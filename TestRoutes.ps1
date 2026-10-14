# Checks each API route's HTTP status code against its expected value. Start the server first (cargo run).
# Usage: .\TestRoutes.ps1 [-BaseUrl http://127.0.0.1:3000]
param([string]$BaseUrl = 'http://127.0.0.1:3000')

# Each entry: HTTP method, path with optional query string, and the status code the route should return.
$tests = @(
    # Basic routes and health checks
    @{ Method = 'GET';  Path = '/';                                                  Expected = 200 },
    @{ Method = 'GET';  Path = '/serverhealth';                                      Expected = 200 },
    @{ Method = 'GET';  Path = '/dbhealth';                                          Expected = 200 },

    # /incidenttickets: limit defaults to 1000 when omitted; invalid limit or field returns 400
    @{ Method = 'GET';  Path = '/incidenttickets';                                   Expected = 200 },
    @{ Method = 'GET';  Path = '/incidenttickets?limit=5';                           Expected = 200 },
    @{ Method = 'GET';  Path = '/incidenttickets?limit=0';                           Expected = 200 },
    @{ Method = 'GET';  Path = '/incidenttickets?limit=abc';                         Expected = 400 },
    @{ Method = 'GET';  Path = '/incidenttickets?limit=-1';                          Expected = 400 },
    @{ Method = 'GET';  Path = '/incidenttickets?field=ticket_number,state';         Expected = 200 },
    @{ Method = 'GET';  Path = '/incidenttickets?field=ticket_number,state&limit=3'; Expected = 200 },
    @{ Method = 'GET';  Path = '/incidenttickets?field=bad_column';                  Expected = 400 },
    @{ Method = 'GET';  Path = '/incidenttickets?field=ticket_number,,state';        Expected = 400 },
    @{ Method = 'GET';  Path = '/incidenttickets?field=';                            Expected = 400 },

    # /servicedesktickets: same rules as /incidenttickets
    @{ Method = 'GET';  Path = '/servicedesktickets';                                Expected = 200 },
    @{ Method = 'GET';  Path = '/servicedesktickets?limit=5';                        Expected = 200 },
    @{ Method = 'GET';  Path = '/servicedesktickets?limit=0';                        Expected = 200 },
    @{ Method = 'GET';  Path = '/servicedesktickets?limit=abc';                      Expected = 400 },
    @{ Method = 'GET';  Path = '/servicedesktickets?limit=-1';                       Expected = 400 },
    @{ Method = 'GET';  Path = '/servicedesktickets?field=ticket_number,priority';   Expected = 200 },
    @{ Method = 'GET';  Path = '/servicedesktickets?field=ticket_number,priority&limit=3'; Expected = 200 },
    @{ Method = 'GET';  Path = '/servicedesktickets?field=bad_column';               Expected = 400 },
    @{ Method = 'GET';  Path = '/servicedesktickets?field=ticket_number,,priority';  Expected = 400 },
    @{ Method = 'GET';  Path = '/servicedesktickets?field=';                         Expected = 400 },

    # /servicerequesttickets: same rules as /incidenttickets
    @{ Method = 'GET';  Path = '/servicerequesttickets';                             Expected = 200 },
    @{ Method = 'GET';  Path = '/servicerequesttickets?limit=5';                     Expected = 200 },
    @{ Method = 'GET';  Path = '/servicerequesttickets?limit=0';                     Expected = 200 },
    @{ Method = 'GET';  Path = '/servicerequesttickets?limit=abc';                   Expected = 400 },
    @{ Method = 'GET';  Path = '/servicerequesttickets?limit=-1';                    Expected = 400 },
    @{ Method = 'GET';  Path = '/servicerequesttickets?field=ticket_number,priority'; Expected = 200 },
    @{ Method = 'GET';  Path = '/servicerequesttickets?field=ticket_number,priority&limit=3'; Expected = 200 },
    @{ Method = 'GET';  Path = '/servicerequesttickets?field=bad_column';            Expected = 400 },
    @{ Method = 'GET';  Path = '/servicerequesttickets?field=ticket_number,,priority'; Expected = 400 },
    @{ Method = 'GET';  Path = '/servicerequesttickets?field=';                      Expected = 400 },

    # Wrong HTTP method (405) and unknown path (404)
    @{ Method = 'POST'; Path = '/serverhealth';                                      Expected = 405 },
    @{ Method = 'POST'; Path = '/incidenttickets';                                   Expected = 405 },
    @{ Method = 'GET';  Path = '/unknownroute';                                      Expected = 404 }
)

$failed = 0
foreach ($t in $tests) {
    try {
        $response = Invoke-WebRequest -Uri ($BaseUrl + $t.Path) -Method $t.Method -UseBasicParsing -TimeoutSec 120
        $actual = [int]$response.StatusCode
    }
    catch {
        # Invoke-WebRequest throws on 4xx/5xx, so read the status from the error response.
        # Status 0 means no response was received (for example, the server is not running).
        if ($_.Exception.Response) { $actual = [int]$_.Exception.Response.StatusCode } else { $actual = 0 }
    }
    $ok = ($actual -eq $t.Expected)
    if (-not $ok) { $failed++ }
    '{0,-5} {1,-62} expected={2} actual={3} {4}' -f $t.Method, $t.Path, $t.Expected, $actual, $(if ($ok) { 'PASS' } else { 'FAIL' })
}
"Failed: $failed of $($tests.Count)"
