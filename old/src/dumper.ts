import fs from "fs";
import path from "path";
import fetch, { Response } from "node-fetch";
import { URL, URLSearchParams } from "url";
import assert from "assert";

const url = new URL(process.argv[2]);

const globalData = JSON.parse(fs.readFileSync(path.join(__dirname, "data"), "utf-8")) as GlobalData;
fs.unlinkSync(path.join(__dirname, "data"));

const assertOk = (res: Response) => (res.ok ? Promise.resolve(res) : Promise.reject(res));

const cookies = () => {};

const getToken = () =>
	fetch(globalData.tokenEndpoint, {
		method: "POST",
		body: new URLSearchParams({
			client_id: globalData.clientId,
			grant_type: "authorization_code",
			redirect_uri: "wilma://oauth",
			code_verifier: globalData.verifier,
			code: url.searchParams.get("code")!,
		}),
	})
		.then(assertOk)
		.then((res) => res.json()) as Promise<TokenResponse>;

const getWilmaCookie = (token: TokenResponse) =>
	fetch(new URL("/index_json", globalData.host).href)
		.then(assertOk)
		.then((res) => res.json() as Promise<WilmaIndexJson>)
		.then(({ SessionID }) =>
			fetch(new URL("/api/v1/external/openid/login", globalData.host).href, {
				method: "POST",
				body: new URLSearchParams({
					payload: JSON.stringify({
						configuration: globalData.configuration,
						clientId: globalData.clientId,
						accessToken: token.access_token,
						sessionId: SessionID,
						idToken: token.id_token,
					}),
				}),
			}),
		)
		.then(assertOk)
		.then((res) => {});

fetch(globalData.tokenEndpoint, {
	method: "POST",
	body: params,
}).then(async (res) => {
	const data = (await res.json()) as TokenResponse;

	const { SessionID } = (await fetch(new URL("/index_json", globalData.host).href).then((res) =>
		res.json(),
	)) as WilmaIndexJson;

	fetch(new URL("/api/v1/external/openid/login", globalData.host).href, {
		method: "POST",
		body: new URLSearchParams({
			payload: JSON.stringify({
				configuration: globalData.configuration,
				clientId: globalData.clientId,
				accessToken: data.access_token,
				sessionId: SessionID,
				idToken: data.id_token,
			}),
		}),
	}).then(async (res) => {
		assert(res.status === 200, "Login failed");
		const cookies = new Map();

		for (const c of res.headers.raw()["set-cookie"]) {
			const [name, value] = c.split(";")[0].split("=");
			cookies.set(name, value);
		}

		fs.writeFileSync(path.join(__dirname, "cookies"), JSON.stringify(Object.fromEntries(cookies)));
	});
});
