import Client from "./client";

const DEFAULT_API_BASE = "http://localhost:4000";

export function createEncoreClient(request: Request): Client {
  let baseUrl: string;
  try {
    const url = new URL(request.url);
    // Encore may use "origin" as internal host when forwarding; it doesn't resolve.
    if (url.hostname === "origin") {
      baseUrl = process.env.ENCORE_API_BASE_URL ?? DEFAULT_API_BASE;
    } else {
      baseUrl = url.origin;
    }
  } catch {
    baseUrl = process.env.ENCORE_API_BASE_URL ?? DEFAULT_API_BASE;
  }
  return new Client(baseUrl);
}
