import { Link } from "react-router";

export function meta() {
  return [
    { title: "Uptime Monitoring" },
    { name: "description", content: "Monitor your websites' uptime" },
  ];
}

export default function Landing() {
  return (
    <div className="min-h-full container px-4 mx-auto my-16">
      <h1 className="text-3xl font-bold leading-tight text-gray-900 dark:text-gray-100">
        Uptime Monitoring
      </h1>
      <p className="mt-4 text-lg text-gray-600 dark:text-gray-400">
        Monitor your websites and get notified when they go down.
      </p>
      <div className="mt-8 flex gap-4">
        <Link
          to="/signup"
          className="inline-flex items-center justify-center rounded-md border border-transparent bg-indigo-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 dark:focus:ring-offset-gray-900"
        >
          Get started
        </Link>
        <Link
          to="/signin"
          className="inline-flex items-center justify-center rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-300 dark:hover:bg-gray-700"
        >
          Sign in
        </Link>
        <Link
          to="/pricing"
          className="inline-flex items-center justify-center rounded-md px-4 py-2 text-sm font-medium text-gray-700 hover:text-gray-900 dark:text-gray-300 dark:hover:text-gray-100"
        >
          Pricing
        </Link>
      </div>
    </div>
  );
}
