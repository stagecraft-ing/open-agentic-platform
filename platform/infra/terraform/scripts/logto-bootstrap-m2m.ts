import fs from "node:fs";

function must(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`Missing env var ${name}`);
  return v;
}

const LOGTO_ADMIN_URL = must("LOGTO_ADMIN_URL"); // ex https://logto-admin.stagecraft.ing
const DEPLOYD_AUDIENCE = must("DEPLOYD_AUDIENCE"); // ex https://api.deployd.xyz
const DEPLOYD_SCOPE = process.env.DEPLOYD_SCOPE ?? "deployd:deploy";

// Where to write outputs for terraform.tfvars
const OUT_PATH = process.env.OUT_PATH ?? "logto.m2m.out.json";

console.log("Logto bootstrap scaffold");
console.log("- Admin URL:", LOGTO_ADMIN_URL);
console.log("- Deployd audience:", DEPLOYD_AUDIENCE);
console.log("- Deployd scope:", DEPLOYD_SCOPE);
console.log("");
console.log("TODO: Wire Logto Management API here to:");
console.log("1) Create API Resource with identifier = DEPLOYD_AUDIENCE");
console.log("2) Create M2M app for stagecraft");
console.log("3) Grant scope DEPLOYD_SCOPE to that app");
console.log("4) Output client_id and client_secret for terraform");

const placeholder = {
  deploydAudience: DEPLOYD_AUDIENCE,
  deploydScope: DEPLOYD_SCOPE,
  logtoM2MClientId: "",
  logtoM2MClientSecret: ""
};

fs.writeFileSync(OUT_PATH, JSON.stringify(placeholder, null, 2));
console.log(`Wrote ${OUT_PATH}. Fill client id and secret from Logto Admin console, then paste into terraform.tfvars.`);
