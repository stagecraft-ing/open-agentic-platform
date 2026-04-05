# Content Page Pattern

Static content page (help, about, terms, etc.). Server Component with no
data fetching — pure markup.

## Template

```tsx
export default function {PageName}Page() {
  return (
    <article className="prose prose-gray dark:prose-invert max-w-3xl mx-auto py-8">
      <h1>{Title}</h1>
      <p>{IntroText}</p>

      <h2>Section One</h2>
      <p>{SectionContent}</p>

      <h2>Section Two</h2>
      <ul>
        <li>{Item1}</li>
        <li>{Item2}</li>
      </ul>
    </article>
  );
}
```

## Rules

1. Server Component — no `"use client"` needed.
2. Use Tailwind `prose` class for typographic styling.
3. No data fetching — all content is static.
4. Can optionally require auth if content is private.
