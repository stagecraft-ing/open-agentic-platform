import { Outlet, NavLink } from "react-router";

const SETTINGS_NAV = [
  { to: "/app/settings", label: "General", end: true },
  { to: "/app/settings/connectors", label: "Connectors", end: false },
  { to: "/app/settings/github-pat", label: "GitHub PAT", end: false },
];

export default function SettingsLayout() {
  return (
    <div>
      <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
        Settings
      </h2>

      <div className="flex gap-1 border-b border-gray-200 dark:border-gray-700 mb-6">
        {SETTINGS_NAV.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.end}
            className={({ isActive }) =>
              `px-3 py-2 text-sm font-medium border-b-2 transition-colors ${
                isActive
                  ? "border-indigo-500 text-indigo-600 dark:text-indigo-400"
                  : "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400"
              }`
            }
          >
            {item.label}
          </NavLink>
        ))}
      </div>

      <Outlet />
    </div>
  );
}
