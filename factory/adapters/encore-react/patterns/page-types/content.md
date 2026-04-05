# Content Page Pattern

## Convention
Static content page. No loader, no data fetching. Pure markup with Tailwind typography.

## Template
```tsx
export default function {PageName}() {
  return (
    <div className="max-w-3xl mx-auto px-8 py-16 prose">
      <h1>{Page Title}</h1>
      <p>{Content paragraph}</p>

      <h2>{Section}</h2>
      <p>{More content}</p>
    </div>
  );
}
```

## Rules
1. No loader, no action, no data
2. Use Tailwind `prose` class for typography
3. Semantic HTML headings
