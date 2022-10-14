import path from "path";
import readline from "readline";
import crypto from "crypto";
import fs from "fs";

import open from "open";
import fetch from "node-fetch";
import ProtocolRegistry from "protocol-registry";
import { URL } from "url";

const nodePath = process.argv[0];

const wilmaUrl = process.argv[2] ?? "https://turku.inschool.fi";
const indexUrl = new URL("/index_json", wilmaUrl).href;

const getLine = () =>
	new Promise<string>((resolve) => {
		const rl = readline.createInterface({
			input: process.stdin,
		});

		rl.once("line", (line) => {
			rl.close();
			resolve(line);
		});
	});

const generateRandomString = () => crypto.randomBytes(64).toString("hex");

const generateCode = () => {
	const verifier = generateRandomString();
	const challenge = crypto
		.createHash("sha256")
		.update(verifier)
		.digest("base64")
		.replace(/\+/g, "-")
		.replace(/\//g, "_")
		.replace(/=/g, "");

	return [verifier, challenge] as const;
};

ProtocolRegistry.register({
	protocol: "wilma",
	command: `"${nodePath}" ${path.join(__dirname, "dumper.js")} "$_URL_"`,
	override: true,
	terminal: true,
}).then(async () => {
	const data = (await fetch(indexUrl).then((res) => res.json())) as WilmaIndexJson;

	const providers = data.oidc_providers;
	providers.forEach(({ name }, i) => console.log(`${i + 1}: ${name}`));
	process.stdout.write("\nSelect provider: ");
	const index = parseInt(await getLine()) - 1;

	const provider = providers[index];
	const configuration = (await fetch(provider.configuration).then((res) => res.json())) as OpenIDConfiguration;

	const { authorization_endpoint } = configuration;

	const [verifier, challenge] = generateCode();

	fs.writeFileSync(
		path.join(__dirname, "data"),
		JSON.stringify({
			clientId: provider.client_id,
			verifier,
			tokenEndpoint: configuration.token_endpoint,
			host: wilmaUrl,
			configuration: provider.configuration,
		}),
	);

	const url = new URL("", authorization_endpoint);
	url.searchParams.set("client_id", provider.client_id);
	url.searchParams.set("redirect_uri", "wilma://oauth");
	url.searchParams.set("response_type", "code");
	url.searchParams.set("scope", provider.scope);
	url.searchParams.set("code_challenge_method", "S256");
	url.searchParams.set("code_challenge", challenge);

	console.log(url.href);

	open(url.href);
});
