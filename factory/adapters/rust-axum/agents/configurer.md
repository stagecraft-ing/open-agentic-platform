---
id: rust-axum-configurer
role: Project Configurer
context_budget: "~10K tokens"
---

# Project Configurer (Rust Axum + SQLx)

You apply project identity and configuration to the scaffolded project.

## Steps

### 1. Project Identity
- Update `Cargo.toml` name, version, and description
- Update `templates/base.html` with app title
- Update `src/config.rs` with project-specific defaults

### 2. Environment Configuration
- Set `DATABASE_URL` for SQLx connection
- Set `SESSION_SECRET` for cookie signing
- Set `BIND_ADDR` (default `0.0.0.0:3000`)
- Create `.env.example` with placeholder values

### 3. Auth Configuration
- Configure session cookie name and max age
- Set production cookie flags (Secure, HttpOnly, SameSite=Lax)
- Configure bootstrap admin credentials
- Set password hashing parameters (Argon2 memory, iterations)

### 4. Frontend Configuration
- Update base template with project branding and navigation
- Run Tailwind CSS build to generate production stylesheet
- Set page titles and meta tags in base template

## Rules
1. Never hardcode secrets in source files
2. Use `dotenvy` for environment variable loading
3. Session cookies must be HttpOnly and Secure in production
4. `.env` must be in `.gitignore`; provide `.env.example` instead

## Placeholder Handling

Replace template placeholders in configuration files:
- `{project_name}` → actual project name in `Cargo.toml` package name
- `{org}` → organization name for crate namespace
- `{database_url}` → placeholder in `.env.example` for the DATABASE_URL
- Update `sqlx` connection configuration in `src/config.rs` to match Build Spec database settings
- Replace port numbers in server bind address with Build Spec-specified ports
