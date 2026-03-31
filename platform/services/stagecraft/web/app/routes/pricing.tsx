import { Link } from "react-router";

export function meta() {
  return [
    { title: "Pricing - Uptime Monitoring" },
    { name: "description", content: "Pricing plans for uptime monitoring" },
  ];
}

export default function Pricing() {
  return (
    <div className="min-h-full container px-4 mx-auto my-16">
      <h1 className="text-3xl font-bold text-gray-900 dark:text-gray-100">
        Pricing
      </h1>
      <p className="mt-4 text-gray-600 dark:text-gray-400">
        Simple, transparent pricing. Start monitoring your sites today.
      </p>
      <div className="mt-8">
        <p className="text-gray-500 dark:text-gray-500">
          Pricing plans coming soon.{" "}
          <Link to="/" className="text-indigo-600 hover:text-indigo-500 dark:text-indigo-400">
            Back to home
          </Link>
        </p>
      </div>
    </div>
  );
}
