interface WilmaIndexJson {
	LoginResult: string;
	SessionID: string;
	ApiVersion: string;
	oidc_providers: Array<OidcProvider>;
}

interface OidcProvider {
	name: string;
	client_id: string;
	configuration: string;
	scope: string;
}

interface OpenIDConfiguration {
	authorization_endpoint: string;
	token_endpoint: string;
}

interface TokenResponse {
	access_token: string;
	id_token: string;
}

interface GlobalData {
	clientId: string;
	verifier: string;
	tokenEndpoint: string;
	host: string;
	configuration: string;
}
