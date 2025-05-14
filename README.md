# Grafana MCP Server Extension for Zed

This extension integrates the [Grafana MCP Server][g-mpc] as a Zed context server extension.

## Installation

1. Create a service account in Grafana with enough permissions to use the tools you want to use,
   generate a service account token, and copy it to the clipboard for use in the configuration file.
   Follow the [Grafana documentation][service-account] for details.

2.  Navigate to the **Extensions** menu in Zed, or use the command palette to search for **extensions**, and search for the **Grafana MCP Server**.

3. In your Zed settings, add configuration for the Grafana MCP server extension:

```json
{
  "context_servers": {
    "mcp-grafana": {
      "settings": {
        "grafana_url": "<your grafana url>",
        "grafana_api_key": "<your service account token>"
      }
    }
  }
}
```

You can alternatively set the `GRAFANA_URL` and `GRAFANA_API_KEY` environment variables to configure the extension.

## License

This project is licensed under the [Apache 2.0 License](LICENSE).

[g-mcp]: https://github.com/grafana/mcp-grafana
[service-account]: https://grafana.com/docs/grafana/latest/administration/service-accounts/
