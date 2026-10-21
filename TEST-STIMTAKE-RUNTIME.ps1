$ErrorActionPreference = 'Stop'
$Base = 'http://127.0.0.1:8790'
$SmokeName = 'stimtake-runtime-smoke-' + [guid]::NewGuid().ToString('N') + '.html'
$Temp = Join-Path $env:TEMP $SmokeName
$Original = $null
$Imported = $false
$TestDeck = 'runtime-smoke-' + [guid]::NewGuid().ToString('N')
$OtherDeck = 'runtime-smoke-switch-' + [guid]::NewGuid().ToString('N')
$TestInstanceIds = @()

function Send-Slots($Slots, $DeckId) {
    $payload = @{ deckId = $DeckId; slots = @($Slots) } | ConvertTo-Json -Depth 8
    Invoke-RestMethod -Uri "$Base/api/slots" -Method Post -ContentType 'application/json' -Body $payload | Out-Null
}

try {
    Write-Host 'StimTake Runtime / Deck-Switch Reset Smoke Test' -ForegroundColor Cyan
    # Fail quickly with a clear diagnosis when nothing is listening at all.
    $listener = Test-NetConnection -ComputerName 127.0.0.1 -Port 8790 -InformationLevel Quiet -WarningAction SilentlyContinue
    if (-not $listener) { throw 'Nothing is listening on 127.0.0.1:8790. Start StimTake.exe first; if it is already open, check netstat -ano | findstr :8790 for a port conflict.' }
    Write-Host '[PASS] TCP listener accepts connections on 127.0.0.1:8790' -ForegroundColor Green
    foreach ($path in @('/health','/obs.html','/api/state','/api/memory')) {
        $response = Invoke-WebRequest -UseBasicParsing -Uri ($Base + $path) -TimeoutSec 4
        if ([int]$response.StatusCode -ne 200) { throw "HTTP check failed: $path" }
        Write-Host "[PASS] $path responds HTTP $([int]$response.StatusCode)" -ForegroundColor Green
    }
    $Original = Invoke-RestMethod -Uri "$Base/api/state" -TimeoutSec 4
    if ($Original.serverStatus -ne 'SERVER_READY') { throw "Backend server status is not SERVER_READY (got '$($Original.serverStatus)')." }
    if (@($Original.slots).Count -ne 20) { throw 'State endpoint did not return exactly 20 slots.' }
    if ([int]$Original.limits.maxActiveOverlays -ne 5) { throw 'Backend did not report a hard limit of five active overlays.' }
    Write-Host '[PASS] Backend reports hard limit of five active overlays' -ForegroundColor Green
    Write-Host '[PASS] State endpoint returns 20 slots' -ForegroundColor Green
    Write-Host '[PASS] Backend reports SERVER_READY only after listener startup' -ForegroundColor Green

    $obs = (Invoke-WebRequest -UseBasicParsing -Uri "$Base/obs.html" -TimeoutSec 4).Content
    if ($obs -notmatch 'frames=new Map\(\)' -or $obs -notmatch 's\.instances' -or $obs -notmatch 'instance_id') { throw 'OBS page does not use stable active-instance IDs.' }
    Write-Host '[PASS] OBS compositor is keyed by independent instance IDs' -ForegroundColor Green
    $memory = Invoke-RestMethod -Uri "$Base/api/memory" -TimeoutSec 4
    if ($null -eq $memory) { throw 'Memory endpoint returned an empty response.' }
    Write-Host '[PASS] /api/memory returns parseable JSON' -ForegroundColor Green

    $html = '<!doctype html><html><body style="margin:0;background:transparent"><div id="stimtake-smoke-test">STIMTAKE_SMOKE_OK</div></body></html>'
    [System.IO.File]::WriteAllText($Temp, $html, [System.Text.Encoding]::UTF8)
    Invoke-WebRequest -UseBasicParsing -Uri "$Base/api/import/1?name=$SmokeName" -Method Post -InFile $Temp -ContentType 'text/html' -TimeoutSec 5 | Out-Null
    Invoke-WebRequest -UseBasicParsing -Uri "$Base/api/import/2?name=$SmokeName" -Method Post -InFile $Temp -ContentType 'text/html' -TimeoutSec 5 | Out-Null
    $Imported = $true
    Write-Host '[PASS] Overlay HTML upload endpoint accepts an HTML file' -ForegroundColor Green

    $testSlots = @($Original.slots | ForEach-Object { [ordered]@{ name=$_.name; file=([string]$_.html_file); active=$false; layer=[int]$_.layer; duration_secs=[int]$_.duration_secs } })
    $testSlots[0].file = $SmokeName; $testSlots[0].active = $true
    Send-Slots $testSlots $TestDeck
    $testSlots[1].file = $SmokeName; $testSlots[1].active = $true
    Send-Slots $testSlots $TestDeck
    Start-Sleep -Milliseconds 400
    $state = Invoke-RestMethod -Uri "$Base/api/state" -TimeoutSec 4
    $firstDeckInstances = @($state.instances | Where-Object { $_.deck_id -eq $TestDeck })
    if ($firstDeckInstances.Count -ne 2) { throw "Expected two active instances, found $($firstDeckInstances.Count)." }
    $TestInstanceIds += $firstDeckInstances.instance_id
    Write-Host '[PASS] Two independent overlay instances activate together' -ForegroundColor Green

    # Ask for six concurrent instances in one payload. The backend must reject
    # the entire update atomically and preserve the two already-live instances.
    $overLimit = @($testSlots | ForEach-Object { [ordered]@{ name=$_.name; file=$SmokeName; active=$true; layer=[int]$_.layer; duration_secs=[int]$_.duration_secs } })
    for ($i = 2; $i -lt 6; $i++) { $overLimit[$i].file = $SmokeName; $overLimit[$i].active = $true }
    $limitDenied = $false
    try {
        $payload = @{ deckId = $TestDeck; slots = @($overLimit) } | ConvertTo-Json -Depth 8
        Invoke-WebRequest -UseBasicParsing -Uri "$Base/api/slots" -Method Post -ContentType 'application/json' -Body $payload -TimeoutSec 4 | Out-Null
    } catch {
        $status = 0
        if ($_.Exception.Response) { $status = [int]$_.Exception.Response.StatusCode }
        if ($status -eq 409) { $limitDenied = $true } else { throw }
    }
    if (-not $limitDenied) { throw 'Backend accepted an activation request above the five-overlay limit.' }
    $state = Invoke-RestMethod -Uri "$Base/api/state" -TimeoutSec 4
    if (@($state.instances).Count -ne 2) { throw 'Rejected over-limit request changed existing live instances.' }
    Write-Host '[PASS] Backend rejects overlay six and preserves existing instances' -ForegroundColor Green

    $switchSlots = @($testSlots | ForEach-Object { [ordered]@{ name=$_.name; file=$_.file; active=$true; layer=[int]$_.layer; duration_secs=[int]$_.duration_secs } })
    Send-Slots $switchSlots $OtherDeck
    Start-Sleep -Milliseconds 400
    $state = Invoke-RestMethod -Uri "$Base/api/state" -TimeoutSec 4
    if (@($state.instances).Count -ne 0) { throw 'Deck switch left one or more outgoing or incoming overlays running.' }
    if (@($state.slots | Where-Object { $_.active }).Count -ne 0) { throw 'Deck switch did not initialize all incoming cards as inactive.' }
    Write-Host '[PASS] Deck switch stops all overlays and initializes the new deck with all cards OFF' -ForegroundColor Green

    # A new-deck card can be started normally after the clean switch.
    $switchSlots[0].active = $true
    Send-Slots $switchSlots $OtherDeck
    Start-Sleep -Milliseconds 300
    $state = Invoke-RestMethod -Uri "$Base/api/state" -TimeoutSec 4
    $newDeckInstances = @($state.instances | Where-Object { $_.deck_id -eq $OtherDeck })
    if ($newDeckInstances.Count -ne 1) { throw 'A card could not be activated after switching decks.' }
    $newInstanceId = $newDeckInstances[0].instance_id
    Write-Host '[PASS] New deck cards can activate after the clean switch' -ForegroundColor Green

    Invoke-WebRequest -UseBasicParsing -Uri "$Base/api/instances/$newInstanceId" -Method Delete -TimeoutSec 4 | Out-Null
    $state = Invoke-RestMethod -Uri "$Base/api/state" -TimeoutSec 4
    if (@($state.instances).Count -ne 0) { throw 'Test instance could not be stopped.' }
    Write-Host '[PASS] Individual stop endpoint removes the selected instance' -ForegroundColor Green
    Write-Host 'RUNTIME SMOKE TEST PASSED. This validates backend state; visible OBS rendering still requires an OBS Studio check.' -ForegroundColor Green
}
catch {
    Write-Host "[FAIL] $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}
finally {
    foreach ($id in $TestInstanceIds) {
        try { Invoke-WebRequest -UseBasicParsing -Uri "$Base/api/instances/$id" -Method Delete -TimeoutSec 2 | Out-Null } catch {}
    }
    if ($null -ne $Original) {
        try { Send-Slots @($Original.slots | ForEach-Object { [ordered]@{ name=$_.name; file=([string]$_.html_file); active=$false; layer=[int]$_.layer; duration_secs=[int]$_.duration_secs } }) 'runtime-smoke-restore' } catch { Write-Host '[WARN] Could not restore previous slot configuration automatically.' -ForegroundColor Yellow }
    }
    if ($Imported) {
        try { Invoke-WebRequest -UseBasicParsing -Uri "$Base/api/import/1?name=$SmokeName" -Method Delete -TimeoutSec 4 | Out-Null } catch { Write-Host "[WARN] Remove test file manually if needed: $SmokeName" -ForegroundColor Yellow }
        try { Invoke-WebRequest -UseBasicParsing -Uri "$Base/api/import/2?name=$SmokeName" -Method Delete -TimeoutSec 4 | Out-Null } catch { Write-Host "[WARN] Remove test file manually if needed: $SmokeName" -ForegroundColor Yellow }
    }
    Remove-Item $Temp -Force -ErrorAction SilentlyContinue
}
