/// @file bridge.cpp
/// @brief Clang LibTooling C++ 桥接实现
///
/// 使用 RecursiveASTVisitor 遍历 C/C++ AST，提取符号和关系，
/// 将结果序列化为 JSON 通过 C FFI 传递给 Rust 端。
///
/// @design 采用策略模式：ASTVisitor 作为具体策略，
///         由 bridge 入口函数根据编译参数选择合适的解析方式。

#include "bridge.h"

#include <cstdlib>
#include <cstring>
#include <memory>
#include <string>
#include <sstream>
#include <vector>

#include "clang/AST/ASTConsumer.h"
#include "clang/AST/ASTContext.h"
#include "clang/AST/ParentMapContext.h"
#include "clang/AST/Decl.h"
#include "clang/AST/DeclCXX.h"
#include "clang/AST/DeclGroup.h"
#include "clang/AST/Expr.h"
#include "clang/AST/RecursiveASTVisitor.h"
#include "clang/AST/Stmt.h"
#include "clang/AST/Type.h"
#include "clang/Basic/SourceLocation.h"
#include "clang/Basic/SourceManager.h"
#include "clang/Frontend/ASTUnit.h"
#include "clang/Frontend/CompilerInstance.h"
#include "clang/Frontend/FrontendAction.h"
#include "clang/Lex/PPCallbacks.h"
#include "clang/Lex/Preprocessor.h"
#include "clang/Tooling/CommonOptionsParser.h"
#include "clang/Tooling/CompilationDatabase.h"
#include "clang/Tooling/Tooling.h"
#include "clang/Basic/Version.h"
#include "llvm/Support/raw_ostream.h"

// ─── JSON 序列化辅助 ──────────────────────────────────────────────

static std::string json_escape(const std::string &s) {
    std::string out;
    out.reserve(s.size() + 16);
    for (char c : s) {
        switch (c) {
        case '"':  out += "\\\""; break;
        case '\\': out += "\\\\"; break;
        case '\n': out += "\\n";  break;
        case '\r': out += "\\r";  break;
        case '\t': out += "\\t";  break;
        default:
            if (static_cast<unsigned char>(c) < 0x20) {
                char buf[8];
                snprintf(buf, sizeof(buf), "\\u%04x", (unsigned)c);
                out += buf;
            } else {
                out += c;
            }
        }
    }
    return out;
}

// ─── 数据结构 ──────────────────────────────────────────────────────

struct SymbolInfo {
    std::string name;
    std::string qualified_name;
    std::string kind;       // "function", "variable", "type", "macro"
    std::string sub_kind;   // "struct", "class", "enum", "union", "typedef"
    std::string file_path;
    unsigned line = 0;
    unsigned column = 0;
    unsigned end_line = 0;
    bool is_definition = false;
    std::string return_type;
    std::string parameters; // JSON array string
    std::string access;     // "public", "protected", "private"
    std::string attributes; // JSON object string

    std::string to_json() const {
        std::ostringstream os;
        os << "{";
        os << "\"name\":\"" << json_escape(name) << "\"";
        os << ",\"qualified_name\":\"" << json_escape(qualified_name) << "\"";
        os << ",\"kind\":\"" << kind << "\"";
        if (!sub_kind.empty())
            os << ",\"sub_kind\":\"" << sub_kind << "\"";
        os << ",\"file_path\":\"" << json_escape(file_path) << "\"";
        os << ",\"line\":" << line;
        os << ",\"column\":" << column;
        if (end_line > 0)
            os << ",\"end_line\":" << end_line;
        os << ",\"is_definition\":" << (is_definition ? "true" : "false");
        if (!return_type.empty())
            os << ",\"return_type\":\"" << json_escape(return_type) << "\"";
        if (!parameters.empty())
            os << ",\"parameters\":" << parameters;
        if (!access.empty())
            os << ",\"access\":\"" << access << "\"";
        if (!attributes.empty())
            os << ",\"attributes\":" << attributes;
        os << "}";
        return os.str();
    }
};

struct CallInfo {
    std::string caller_name;
    std::string callee_name;
    std::string file_path;
    unsigned line = 0;
    unsigned column = 0;
    bool is_virtual = false;
    bool is_indirect = false;

    std::string to_json() const {
        std::ostringstream os;
        os << "{";
        os << "\"caller\":\"" << json_escape(caller_name) << "\"";
        os << ",\"callee\":\"" << json_escape(callee_name) << "\"";
        os << ",\"file\":\"" << json_escape(file_path) << "\"";
        os << ",\"line\":" << line;
        os << ",\"column\":" << column;
        os << ",\"is_virtual\":" << (is_virtual ? "true" : "false");
        os << ",\"is_indirect\":" << (is_indirect ? "true" : "false");
        os << "}";
        return os.str();
    }
};

struct IncludeInfo {
    std::string source_file;
    std::string target_file;
    unsigned line = 0;
    bool is_system = false;
    std::string resolved_path;

    std::string to_json() const {
        std::ostringstream os;
        os << "{";
        os << "\"source_file\":\"" << json_escape(source_file) << "\"";
        os << ",\"target_file\":\"" << json_escape(target_file) << "\"";
        os << ",\"line\":" << line;
        os << ",\"is_system\":" << (is_system ? "true" : "false");
        if (!resolved_path.empty())
            os << ",\"resolved_path\":\"" << json_escape(resolved_path) << "\"";
        os << "}";
        return os.str();
    }
};

struct InheritInfo {
    std::string derived;
    std::string base;
    std::string access;
    bool is_virtual = false;

    std::string to_json() const {
        std::ostringstream os;
        os << "{";
        os << "\"derived\":\"" << json_escape(derived) << "\"";
        os << ",\"base\":\"" << json_escape(base) << "\"";
        os << ",\"access\":\"" << access << "\"";
        os << ",\"is_virtual\":" << (is_virtual ? "true" : "false");
        os << "}";
        return os.str();
    }
};

struct ReferenceInfo {
    std::string symbol_name;
    std::string file_path;
    unsigned line = 0;
    unsigned column = 0;
    std::string ref_kind; // "read", "write", "address", "type"

    std::string to_json() const {
        std::ostringstream os;
        os << "{";
        os << "\"symbol_name\":\"" << json_escape(symbol_name) << "\"";
        os << ",\"file\":\"" << json_escape(file_path) << "\"";
        os << ",\"line\":" << line;
        os << ",\"column\":" << column;
        os << ",\"ref_kind\":\"" << ref_kind << "\"";
        os << "}";
        return os.str();
    }
};

struct ParseData {
    std::string main_file;
    std::vector<SymbolInfo> symbols;
    std::vector<CallInfo> calls;
    std::vector<IncludeInfo> includes;
    std::vector<InheritInfo> inherits;
    std::vector<ReferenceInfo> references;

    std::string to_json() const {
        std::ostringstream os;
        os << "{";

        os << "\"symbols\":[";
        for (size_t i = 0; i < symbols.size(); ++i) {
            if (i > 0) os << ",";
            os << symbols[i].to_json();
        }
        os << "]";

        os << ",\"calls\":[";
        for (size_t i = 0; i < calls.size(); ++i) {
            if (i > 0) os << ",";
            os << calls[i].to_json();
        }
        os << "]";

        os << ",\"includes\":[";
        for (size_t i = 0; i < includes.size(); ++i) {
            if (i > 0) os << ",";
            os << includes[i].to_json();
        }
        os << "]";

        os << ",\"inherits\":[";
        for (size_t i = 0; i < inherits.size(); ++i) {
            if (i > 0) os << ",";
            os << inherits[i].to_json();
        }
        os << "]";

        os << ",\"references\":[";
        for (size_t i = 0; i < references.size(); ++i) {
            if (i > 0) os << ",";
            os << references[i].to_json();
        }
        os << "]";

        os << "}";
        return os.str();
    }
};

// ─── AST Visitor ───────────────────────────────────────────────────

static std::string get_access_str(clang::AccessSpecifier AS) {
    switch (AS) {
    case clang::AS_public:    return "public";
    case clang::AS_protected: return "protected";
    case clang::AS_private:   return "private";
    default:                  return "";
    }
}

class CctASTVisitor : public clang::RecursiveASTVisitor<CctASTVisitor> {
public:
    explicit CctASTVisitor(clang::ASTContext &ctx, ParseData &data,
                           const std::string &main_file)
        : ctx_(ctx), sm_(ctx.getSourceManager()), data_(data),
          main_file_(main_file) {}

    bool VisitFunctionDecl(clang::FunctionDecl *FD) {
        if (!is_in_main_file(FD->getLocation()))
            return true;

        SymbolInfo sym;
        sym.name = FD->getNameAsString();
        sym.qualified_name = FD->getQualifiedNameAsString();
        sym.kind = "function";
        sym.is_definition = FD->isThisDeclarationADefinition();

        fill_location(FD->getLocation(), sym);
        if (FD->isThisDeclarationADefinition() && FD->getBody()) {
            auto end = FD->getBody()->getEndLoc();
            if (end.isValid())
                sym.end_line = sm_.getSpellingLineNumber(end);
        }

        sym.return_type = FD->getReturnType().getAsString();

        std::ostringstream params;
        params << "[";
        for (unsigned i = 0; i < FD->getNumParams(); ++i) {
            auto *P = FD->getParamDecl(i);
            if (i > 0) params << ",";
            params << "[\"" << json_escape(P->getType().getAsString()) << "\",\""
                   << json_escape(P->getNameAsString()) << "\"]";
        }
        params << "]";
        sym.parameters = params.str();

        if (auto *MD = llvm::dyn_cast<clang::CXXMethodDecl>(FD)) {
            sym.access = get_access_str(MD->getAccess());
            std::ostringstream attrs;
            attrs << "{\"is_virtual\":" << (MD->isVirtual() ? "true" : "false")
                  << ",\"is_static\":" << (MD->isStatic() ? "true" : "false")
                  << ",\"is_inline\":" << (FD->isInlined() ? "true" : "false")
                  << "}";
            sym.attributes = attrs.str();
        }

        data_.symbols.push_back(std::move(sym));
        return true;
    }

    bool VisitVarDecl(clang::VarDecl *VD) {
        if (!is_in_main_file(VD->getLocation()))
            return true;
        if (llvm::isa<clang::ParmVarDecl>(VD))
            return true;

        SymbolInfo sym;
        sym.name = VD->getNameAsString();
        sym.qualified_name = VD->getQualifiedNameAsString();
        sym.kind = "variable";
        sym.is_definition = VD->isThisDeclarationADefinition() != clang::VarDecl::DeclarationOnly;

        fill_location(VD->getLocation(), sym);
        sym.return_type = VD->getType().getAsString();

        std::string scope;
        if (VD->isLocalVarDecl()) scope = "local";
        else if (VD->isStaticDataMember()) scope = "member";
        else scope = "global";

        std::ostringstream attrs;
        attrs << "{\"type_name\":\"" << json_escape(VD->getType().getAsString())
              << "\",\"scope\":\"" << scope
              << "\",\"is_const\":" << (VD->getType().isConstQualified() ? "true" : "false")
              << "}";
        sym.attributes = attrs.str();

        if (auto *FD = llvm::dyn_cast<clang::FieldDecl>(VD))
            sym.access = get_access_str(FD->getAccess());

        data_.symbols.push_back(std::move(sym));
        return true;
    }

    bool VisitFieldDecl(clang::FieldDecl *FD) {
        if (!is_in_main_file(FD->getLocation()))
            return true;

        SymbolInfo sym;
        sym.name = FD->getNameAsString();
        sym.qualified_name = FD->getQualifiedNameAsString();
        sym.kind = "variable";
        sym.sub_kind = "member";
        sym.is_definition = true;
        fill_location(FD->getLocation(), sym);
        sym.return_type = FD->getType().getAsString();
        sym.access = get_access_str(FD->getAccess());

        data_.symbols.push_back(std::move(sym));
        return true;
    }

    bool VisitRecordDecl(clang::RecordDecl *RD) {
        if (!is_in_main_file(RD->getLocation()))
            return true;
        if (RD->isImplicit())
            return true;

        SymbolInfo sym;
        sym.name = RD->getNameAsString();
        sym.qualified_name = RD->getQualifiedNameAsString();
        sym.kind = "type";
        sym.is_definition = RD->isCompleteDefinition();
        fill_location(RD->getLocation(), sym);

        if (RD->isStruct()) sym.sub_kind = "struct";
        else if (RD->isClass()) sym.sub_kind = "class";
        else if (RD->isUnion()) sym.sub_kind = "union";

        if (auto *CXXRD = llvm::dyn_cast<clang::CXXRecordDecl>(RD)) {
            if (CXXRD->isCompleteDefinition()) {
                for (const auto &base : CXXRD->bases()) {
                    InheritInfo inh;
                    inh.derived = CXXRD->getQualifiedNameAsString();
                    inh.base = base.getType().getAsString();
                    inh.access = get_access_str(base.getAccessSpecifier());
                    inh.is_virtual = base.isVirtual();
                    data_.inherits.push_back(std::move(inh));
                }
            }
        }

        data_.symbols.push_back(std::move(sym));
        return true;
    }

    bool VisitEnumDecl(clang::EnumDecl *ED) {
        if (!is_in_main_file(ED->getLocation()))
            return true;

        SymbolInfo sym;
        sym.name = ED->getNameAsString();
        sym.qualified_name = ED->getQualifiedNameAsString();
        sym.kind = "type";
        sym.sub_kind = "enum";
        sym.is_definition = ED->isCompleteDefinition();
        fill_location(ED->getLocation(), sym);

        data_.symbols.push_back(std::move(sym));
        return true;
    }

    bool VisitTypedefNameDecl(clang::TypedefNameDecl *TD) {
        if (!is_in_main_file(TD->getLocation()))
            return true;
        if (TD->isImplicit())
            return true;

        SymbolInfo sym;
        sym.name = TD->getNameAsString();
        sym.qualified_name = TD->getQualifiedNameAsString();
        sym.kind = "type";
        sym.sub_kind = "typedef";
        sym.is_definition = true;
        fill_location(TD->getLocation(), sym);
        sym.return_type = TD->getUnderlyingType().getAsString();

        data_.symbols.push_back(std::move(sym));
        return true;
    }

    bool VisitCallExpr(clang::CallExpr *CE) {
        if (!is_in_main_file(CE->getBeginLoc()))
            return true;

        auto *callee_decl = CE->getDirectCallee();
        if (!callee_decl)
            return true;

        auto *enclosing = get_enclosing_function(CE);
        if (!enclosing)
            return true;

        CallInfo ci;
        ci.caller_name = enclosing->getQualifiedNameAsString();
        ci.callee_name = callee_decl->getQualifiedNameAsString();
        ci.line = sm_.getSpellingLineNumber(CE->getBeginLoc());
        ci.column = sm_.getSpellingColumnNumber(CE->getBeginLoc());
        ci.file_path = get_filename(CE->getBeginLoc());

        if (auto *MCE = llvm::dyn_cast<clang::CXXMemberCallExpr>(CE)) {
            auto *MD = MCE->getMethodDecl();
            if (MD && MD->isVirtual())
                ci.is_virtual = true;
        }

        data_.calls.push_back(std::move(ci));
        return true;
    }

    bool VisitDeclRefExpr(clang::DeclRefExpr *DRE) {
        if (!is_in_main_file(DRE->getLocation()))
            return true;

        auto *decl = DRE->getDecl();
        if (!decl)
            return true;

        // Skip function call references (already captured by VisitCallExpr)
        if (llvm::isa<clang::FunctionDecl>(decl)) {
            auto parents = ctx_.getParents(*DRE);
            if (!parents.empty()) {
                if (parents[0].get<clang::CallExpr>() ||
                    parents[0].get<clang::CXXMemberCallExpr>())
                    return true;
            }
        }

        // Skip parameter declarations
        if (llvm::isa<clang::ParmVarDecl>(decl))
            return true;

        ReferenceInfo ri;
        ri.symbol_name = decl->getQualifiedNameAsString();
        ri.file_path = get_filename(DRE->getLocation());
        ri.line = sm_.getSpellingLineNumber(DRE->getLocation());
        ri.column = sm_.getSpellingColumnNumber(DRE->getLocation());

        // Determine reference kind by checking parent context
        auto parents = ctx_.getParents(*DRE);
        if (!parents.empty()) {
            if (auto *BO = parents[0].get<clang::BinaryOperator>()) {
                if (BO->isAssignmentOp() && BO->getLHS() == DRE)
                    ri.ref_kind = "write";
                else
                    ri.ref_kind = "read";
            } else if (parents[0].get<clang::UnaryOperator>()) {
                auto *UO = parents[0].get<clang::UnaryOperator>();
                if (UO->getOpcode() == clang::UO_AddrOf)
                    ri.ref_kind = "address";
                else if (UO->isIncrementDecrementOp())
                    ri.ref_kind = "write";
                else
                    ri.ref_kind = "read";
            } else {
                ri.ref_kind = "read";
            }
        } else {
            ri.ref_kind = "read";
        }

        data_.references.push_back(std::move(ri));
        return true;
    }

private:
    clang::ASTContext &ctx_;
    clang::SourceManager &sm_;
    ParseData &data_;
    std::string main_file_;

    bool is_in_main_file(clang::SourceLocation loc) {
        if (loc.isInvalid()) return false;
        auto fid = sm_.getFileID(sm_.getSpellingLoc(loc));
        return fid == sm_.getMainFileID();
    }

    void fill_location(clang::SourceLocation loc, SymbolInfo &sym) {
        sym.file_path = get_filename(loc);
        sym.line = sm_.getSpellingLineNumber(loc);
        sym.column = sm_.getSpellingColumnNumber(loc);
    }

    std::string get_filename(clang::SourceLocation loc) {
        auto spelling_loc = sm_.getSpellingLoc(loc);
        auto fname = sm_.getFilename(spelling_loc);
        return fname.str();
    }

    clang::FunctionDecl *get_enclosing_function(clang::Stmt *S) {
        auto parents = ctx_.getParents(*S);
        while (!parents.empty()) {
            auto &P = parents[0];
            if (auto *FD = P.get<clang::FunctionDecl>())
                return const_cast<clang::FunctionDecl *>(FD);
            if (auto *stmt = P.get<clang::Stmt>()) {
                parents = ctx_.getParents(*stmt);
            } else if (auto *decl = P.get<clang::Decl>()) {
                if (auto *FD = llvm::dyn_cast<clang::FunctionDecl>(decl))
                    return const_cast<clang::FunctionDecl *>(FD);
                parents = ctx_.getParents(*decl);
            } else {
                break;
            }
        }
        return nullptr;
    }
};

// ─── Include 回调 ──────────────────────────────────────────────────

class CctPPCallbacks : public clang::PPCallbacks {
public:
    CctPPCallbacks(clang::SourceManager &sm, ParseData &data)
        : sm_(sm), data_(data) {}

    void InclusionDirective(
        clang::SourceLocation HashLoc,
        const clang::Token &,
        llvm::StringRef FileName,
        bool IsAngled,
        clang::CharSourceRange,
        clang::OptionalFileEntryRef File,
        llvm::StringRef,
        llvm::StringRef,
        const clang::Module *,
        clang::SrcMgr::CharacteristicKind) override
    {
        if (!sm_.isInMainFile(HashLoc))
            return;

        IncludeInfo inc;
        inc.source_file = sm_.getFilename(sm_.getSpellingLoc(HashLoc)).str();
        inc.target_file = FileName.str();
        inc.line = sm_.getSpellingLineNumber(HashLoc);
        inc.is_system = IsAngled;
        if (File)
            inc.resolved_path = File->getName().str();

        data_.includes.push_back(std::move(inc));
    }

private:
    clang::SourceManager &sm_;
    ParseData &data_;
};

// ─── AST Consumer + FrontendAction ─────────────────────────────────

class CctASTConsumer : public clang::ASTConsumer {
public:
    CctASTConsumer(clang::ASTContext &ctx, ParseData &data,
                   const std::string &main_file)
        : visitor_(ctx, data, main_file) {}

    void HandleTranslationUnit(clang::ASTContext &ctx) override {
        visitor_.TraverseDecl(ctx.getTranslationUnitDecl());
    }

private:
    CctASTVisitor visitor_;
};

class CctFrontendAction : public clang::ASTFrontendAction {
public:
    CctFrontendAction(ParseData &data) : data_(data) {}

    std::unique_ptr<clang::ASTConsumer>
    CreateASTConsumer(clang::CompilerInstance &CI,
                      llvm::StringRef InFile) override {
        auto &PP = CI.getPreprocessor();
        PP.addPPCallbacks(std::make_unique<CctPPCallbacks>(
            CI.getSourceManager(), data_));

        return std::make_unique<CctASTConsumer>(
            CI.getASTContext(), data_, InFile.str());
    }

private:
    ParseData &data_;
};

// ─── Action 工厂 ──────────────────────────────────────────────────

class CctActionFactory : public clang::tooling::FrontendActionFactory {
public:
    CctActionFactory(ParseData &data) : data_(data) {}

    std::unique_ptr<clang::FrontendAction> create() override {
        return std::make_unique<CctFrontendAction>(data_);
    }

private:
    ParseData &data_;
};

// ─── C FFI 入口 ────────────────────────────────────────────────────

extern "C" {

int32_t cct_parse_file(
    const char *file_path,
    const char *compile_db_dir,
    const char *extra_args,
    char **out_json,
    uint64_t *out_json_len)
{
    if (!file_path || !out_json || !out_json_len)
        return -1;

    *out_json = nullptr;
    *out_json_len = 0;

    std::string filepath(file_path);
    ParseData data;
    data.main_file = filepath;

    std::unique_ptr<clang::tooling::CompilationDatabase> comp_db;
    std::string error_msg;

    if (compile_db_dir && compile_db_dir[0] != '\0') {
        comp_db = clang::tooling::CompilationDatabase::loadFromDirectory(
            compile_db_dir, error_msg);
    }

    if (!comp_db) {
        // 无编译数据库时使用默认编译参数
        std::vector<std::string> args;
        args.push_back("clang");
        args.push_back("-std=c++17");
        args.push_back("-fsyntax-only");

        // 解析额外编译参数
        if (extra_args) {
            const char *p = extra_args;
            while (*p) {
                std::string arg(p);
                if (!arg.empty())
                    args.push_back(arg);
                p += arg.size() + 1;
            }
        }

        args.push_back(filepath);
        {
            int argc = static_cast<int>(args.size());
            std::vector<const char *> argv;
            for (auto &a : args) argv.push_back(a.c_str());
            comp_db = clang::tooling::FixedCompilationDatabase::loadFromCommandLine(
                argc, argv.data(), error_msg);
        }
        if (!comp_db) {
            // 最终回退：手工创建 FixedCompilationDatabase
            std::vector<std::string> cmd_args = {"-std=c++17", "-fsyntax-only"};
            comp_db = std::make_unique<clang::tooling::FixedCompilationDatabase>(
                ".", cmd_args);
        }
    }

    std::vector<std::string> sources = {filepath};
    clang::tooling::ClangTool tool(*comp_db, sources);

    CctActionFactory factory(data);
    int result = tool.run(&factory);

    std::string json = data.to_json();

    *out_json_len = json.size();
    *out_json = (char *)malloc(json.size() + 1);
    if (*out_json) {
        memcpy(*out_json, json.c_str(), json.size());
        (*out_json)[json.size()] = '\0';
    }

    return result;
}

void cct_free_string(char *ptr) {
    free(ptr);
}

const char *cct_clang_version(void) {
    return CLANG_VERSION_STRING;
}

} // extern "C"
