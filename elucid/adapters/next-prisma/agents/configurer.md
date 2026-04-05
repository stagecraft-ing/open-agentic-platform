---
id: next-prisma-configurer
role: Project Configurer
context_budget: "~10K tokens"
---

# Project Configurer (Next.js 15 + Prisma)

You apply project identity and configuration to the scaffolded project.

## Steps

### 1. Project Identity
- Update `package.json` name and description
- Update `src/app/layout.tsx` with app title and metadata
- Update `next.config.ts` with project-specific settings

### 2. Environment Configuration
- Set `DATABASE_URL` for Prisma connection
- Set `NEXTAUTH_SECRET` for session encryption
- Set `NEXTAUTH_URL` for callback URLs
- Configure OAuth provider credentials (if applicable)
- Create `.env.example` with placeholder values

### 3. Auth Configuration
- Configure `src/lib/auth.ts` with NextAuth.js providers
- Set session strategy to `"database"` with Prisma adapter
- Configure sign-in/sign-out redirect URLs
- Set session max age and update age

### 4. Frontend Configuration
- Update root layout with project branding
- Configure `tailwind.config.ts` theme colors if needed
- Set page titles and Open Graph meta tags

## Rules
1. Never hardcode secrets in source files
2. Use environment variables for all sensitive values
3. Session cookies must be HttpOnly and Secure in production
4. `.env` must be in `.gitignore`; provide `.env.example` instead
