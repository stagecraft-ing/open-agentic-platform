# Landing Page Pattern

## Convention
Public landing page with hero section, feature highlights, and call-to-action. No auth required. Tailwind styling.

## Template
```tsx
import { Link } from "react-router";

export default function Landing() {
  return (
    <div className="min-h-screen bg-white">
      {/* Hero */}
      <div className="max-w-4xl mx-auto px-8 py-24 text-center">
        <h1 className="text-5xl font-bold tracking-tight text-gray-900">
          {Service Name}
        </h1>
        <p className="mt-6 text-lg text-gray-600 max-w-2xl mx-auto">
          {Service description}
        </p>
        <div className="mt-10 flex gap-4 justify-center">
          <Link to="/signup"
            className="bg-indigo-600 text-white px-6 py-3 rounded-lg hover:bg-indigo-700">
            Get Started
          </Link>
          <Link to="/signin"
            className="border border-gray-300 px-6 py-3 rounded-lg hover:bg-gray-50">
            Sign In
          </Link>
        </div>
      </div>

      {/* Features */}
      <div className="max-w-4xl mx-auto px-8 py-16">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          {/* Feature cards */}
        </div>
      </div>
    </div>
  );
}
```

## Rules
1. No loader needed — public page
2. Use `<Link>` for navigation, not `<a>`
3. Responsive grid with Tailwind breakpoints
4. Indigo primary color for CTAs
