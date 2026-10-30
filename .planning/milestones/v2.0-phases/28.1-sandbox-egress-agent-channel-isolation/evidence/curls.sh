for p in /v1/mcp_servers /v1/code/triggers /v1/code/sessions /v1/environments/bridge; do
  echo "GET $p -> $(curl -s -o /dev/null -w '%{http_code}' -H 'Authorization: Bearer dummy' -H 'anthropic-beta: oauth-2025-04-20' -H 'anthropic-version: 2023-06-01' http://127.0.0.1:3128$p)"
done
