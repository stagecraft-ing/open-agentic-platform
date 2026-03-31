import { SQLDatabase } from "encore.dev/storage/sqldb";
import { drizzle } from "drizzle-orm/node-postgres";

const AuthDB = new SQLDatabase("auth", {
  migrations: "./migrations",
});

export const db = drizzle(AuthDB.connectionString);
