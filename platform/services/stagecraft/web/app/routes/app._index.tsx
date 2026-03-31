import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import Client, { monitor, site } from "../lib/client";
import { type FC, useEffect, useState } from "react";
import { DateTime } from "luxon";

export default function Dashboard() {
  const [baseURL, setBaseURL] = useState("");
  useEffect(() => setBaseURL(window.location.origin), []);

  if (!baseURL) return null;

  return (
    <div>
      <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-4">
        User dashboard
      </h3>
      <p className="text-gray-600 dark:text-gray-400 mb-6">
        Monitor your websites&apos; uptime below.
      </p>
      <SiteList client={new Client(baseURL)} />
    </div>
  );
}

const SiteList: FC<{ client: Client }> = ({ client }) => {
  const { isLoading, error, data } = useQuery({
    queryKey: ["sites"],
    queryFn: () => client.site.list(),
    refetchInterval: 10000,
    retry: false,
  });

  const { data: status } = useQuery({
    queryKey: ["status"],
    queryFn: () => client.monitor.status(),
    refetchInterval: 1000,
    retry: false,
  });

  const queryClient = useQueryClient();

  const doDelete = useMutation({
    mutationFn: (s: site.Site) => client.site.del(s.id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["sites"] });
    },
  });

  if (isLoading) {
    return <div>Loading...</div>;
  }
  if (error) {
    return (
      <div className="text-red-600 dark:text-red-400">
        {(error as Error).message}
      </div>
    );
  }

  return (
    <>
      <div className="sm:flex sm:items-center">
        <div className="sm:flex-auto">
          <h4 className="text-base font-semibold text-gray-900 dark:text-gray-100">
            Monitored Websites
          </h4>
          <p className="mt-1 text-sm text-gray-700 dark:text-gray-300">
            A list of all the websites being monitored, their current status, and
            when they were last checked.
          </p>
        </div>
        <div className="mt-4 sm:mt-0 sm:ml-16 sm:flex-none">
          <AddSiteForm client={client} />
        </div>
      </div>

      <div className="mt-8 flex flex-col">
        <div className="-my-2 -mx-4 overflow-x-auto sm:-mx-6 lg:-mx-8">
          <div className="inline-block min-w-full py-2 align-middle md:px-6 lg:px-8">
            <div className="overflow-hidden shadow ring-1 ring-black ring-opacity-5 md:rounded-lg dark:ring-white/10">
              <table className="min-w-full divide-y divide-gray-300 dark:divide-gray-600">
                <thead className="bg-gray-50 dark:bg-gray-800">
                  <tr>
                    <th
                      scope="col"
                      className="px-3 py-3.5 text-left text-sm font-semibold text-gray-900 dark:text-gray-100"
                    >
                      Site
                    </th>
                    <th
                      scope="col"
                      className="relative py-3.5 pl-3 pr-4 sm:pr-6"
                    >
                      <span className="sr-only"></span>
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-200 bg-white dark:divide-gray-700 dark:bg-gray-900">
                  {data?.sites.length === 0 && (
                    <tr>
                      <td
                        colSpan={2}
                        className="text-center text-gray-400 py-8 dark:text-gray-500"
                      >
                        Nothing to monitor yet. Add a website to see it here.
                      </td>
                    </tr>
                  )}
                  {data!.sites.map((s) => {
                    const st = status?.sites.find((x) => x.id === s.id);
                    const dt = st && DateTime.fromISO(st.checkedAt);
                    return (
                      <tr key={s.id}>
                        <td className="px-3 py-4 text-sm">
                          <div className="flex items-center gap-2">
                            <span className="text-gray-700 dark:text-gray-300">
                              {s.url}
                            </span>
                            <StatusBadge status={st} />
                          </div>
                          {dt && (
                            <div className="text-gray-400 dark:text-gray-500">
                              Last checked <TimeDelta dt={dt} />
                            </div>
                          )}
                        </td>
                        <td className="relative whitespace-nowrap py-4 pl-3 pr-4 text-right text-sm font-medium sm:pr-6">
                          <button
                            className="text-indigo-600 hover:text-indigo-900 dark:text-indigo-400 dark:hover:text-indigo-300"
                            onClick={() => doDelete.mutate(s)}
                          >
                            Delete
                            <span className="sr-only"> {s.url}</span>
                          </button>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>
    </>
  );
};

const AddSiteForm: FC<{ client: Client }> = ({ client }) => {
  const [formOpen, setFormOpen] = useState(false);
  const [url, setUrl] = useState("");

  const queryClient = useQueryClient();

  const save = useMutation({
    mutationFn: async (url: string) => {
      if (!validURL(url)) return;
      await client.site.add({ url });
      setFormOpen(false);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["sites"] });
      queryClient.invalidateQueries({ queryKey: ["status"] });
    },
  });

  const onSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    save.mutate(url);
  };

  if (!formOpen) {
    return (
      <button
        type="button"
        className="inline-flex items-center justify-center rounded-md border border-transparent bg-indigo-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 sm:w-auto dark:focus:ring-offset-gray-900"
        onClick={() => setFormOpen(true)}
      >
        Add website
      </button>
    );
  }

  return (
    <form onSubmit={onSubmit}>
      <div className="flex flex-col md:flex-row md:items-end gap-4">
        <div>
          <input
            type="text"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="google.com"
            className="mt-1 block w-full rounded-md border-gray-300 p-2 border shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
          />
        </div>

        <div>
          <button
            type="submit"
            className="inline-flex justify-center rounded-md border border-transparent bg-indigo-600 py-2 px-4 text-sm font-medium text-white shadow-sm enabled:hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 disabled:opacity-75 dark:focus:ring-offset-gray-900"
            disabled={!validURL(url)}
          >
            Save
          </button>
        </div>
      </div>
    </form>
  );
};

function validURL(url: string): boolean {
  const idx = url.lastIndexOf(".");
  if (idx === -1 || url.substring(idx + 1) === "") return false;

  if (!url.startsWith("http:") && !url.startsWith("https:")) {
    url = "https://" + url;
  }

  try {
    const u = new URL(url);
    return u.protocol === "http:" || u.protocol === "https:";
  } catch {
    return false;
  }
}

const StatusBadge: FC<{ status: monitor.SiteStatus | undefined }> = ({
  status,
}) => {
  const up = status?.up;
  return up ? (
    <Badge color="green">Up</Badge>
  ) : up === false ? (
    <Badge color="red">Down</Badge>
  ) : (
    <Badge color="gray">Unknown</Badge>
  );
};

const Badge: FC<{
  color: "green" | "red" | "orange" | "gray";
  children?: React.ReactNode;
}> = ({ color, children }) => {
  const [bgColor, textColor] = {
    green: ["bg-green-100", "text-green-800"],
    red: ["bg-red-100", "text-red-800"],
    orange: ["bg-orange-100", "text-orange-800"],
    gray: ["bg-gray-100", "text-gray-800"],
  }[color]!;

  return (
    <span
      className={`inline-flex items-center rounded-md px-2.5 py-0.5 text-sm font-medium uppercase ${bgColor} ${textColor}`}
    >
      {children}
    </span>
  );
};

const TimeDelta: FC<{ dt: DateTime }> = ({ dt }) => {
  const compute = () => dt.toRelative();
  const [str, setStr] = useState(compute());

  useEffect(() => {
    const handler = () => setStr(compute());
    const timer = setInterval(handler, 1000);
    return () => clearInterval(timer);
  }, [dt]);

  return <>{str}</>;
};
