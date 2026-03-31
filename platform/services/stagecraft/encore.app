{
	"id":   "fullstack-app-37a2",
	"lang": "typescript",
	"build": {
		"docker": {
			"bundle_source": true
		}
	},
	"global_cors": {
      "debug": true,

      // Unauthenticated requests (no cookies/HTTP auth/client certs)
      "allow_origins_without_credentials": ["*"],

      // Authenticated requests (cookies or Authorization header)
      "allow_origins_with_credentials": [
        "https://localdev.online",
        "https://*.localdev.online",
        "https://stagecraft.ing",
        "https://*.stagecraft.ing",
        "https://deployd.xyz",
        "https://*.deployd.xyz",
      ],

      // Let the browser send these request headers
      "allow_headers": [
        "Authorization",
        "Content-Type",
        "X-Requested-With"
      ],

      // Only add what you actually read from JS (optional)
      "expose_headers": [
        "Content-Length",
        "Content-Type",
        "Location",
        "Link",
        "X-Request-Id"
      ]
    }
}
