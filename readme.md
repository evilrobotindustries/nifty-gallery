

# Local Metadata
Local metadata can be served via a web server such as [Static Web Server](https://sws.joseluisq.net).

    static-web-server --log-level info \
                      --cache-control-headers false \
                      --directory-listing true \
                      --cors-allow-origins * \
                      --root . \
                      --port 8787

You will then be able to browse your content via http://localhost:8787, assuming you used the same port as above. More information at https://sws.joseluisq.net/configuration/command-line-arguments/