param([Parameter(Mandatory = $true)][string]$InputSource)
$ErrorActionPreference = 'Stop'
$source = Get-Content -LiteralPath $InputSource -Raw -Encoding UTF8
function Get-Section([string]$Start, [string]$End) {
    $a = $source.IndexOf($Start, [StringComparison]::Ordinal)
    if ($a -lt 0) { throw "Missing source section: $Start" }
    $b = $source.IndexOf($End, $a, [StringComparison]::Ordinal)
    if ($b -lt 0) { throw "Missing source boundary: $End" }
    return $source.Substring($a, $b - $a)
}
$types = Get-Section 'enum ClaudeLaunchEntry {' '#[derive(Clone, Debug, PartialEq, Eq, Serialize)]'
$plan = Get-Section 'fn execute_claude_launch_plan<' 'fn sanitized_claude_launch_error('
$sanitize = Get-Section 'fn sanitized_claude_launch_error(' '#[cfg(windows)]'
$recovery = ''
$recoverCall = 'outcome'
if ($source.Contains('fn recover_failed_claude_launch<')) {
    $recovery = Get-Section 'fn recover_failed_claude_launch<' '#[cfg(windows)]'
    $recoverCall = @'
recover_failed_claude_launch(outcome, || {
    recoveries.set(recoveries.get() + 1);
    Some(Ok(()))
}, || execute_claude_launch_plan(&plan, |_| {
    launches.set(launches.get() + 1);
    Ok(())
}, || ClaudeLaunchObservation {
    process_ids: vec![4242], visible_process_id: Some(4242),
    window_titles: vec!["Claude".into()],
}, || {}, 1, 0))
'@
}
$main = @'
fn main() {
    let launches = std::cell::Cell::new(0);
    let recoveries = std::cell::Cell::new(0);
    let plan = vec![ClaudeLaunchEntry::PackagedApp("Claude_family!Claude".into()),
        ClaudeLaunchEntry::AppsFolderApp("Claude_family!Claude".into())];
    let outcome = execute_claude_launch_plan(&plan, |entry| {
        launches.set(launches.get() + 1);
        match entry { ClaudeLaunchEntry::PackagedApp(_) => Err("HRESULT 0x80070005".into()), _ => Ok(()) }
    }, ClaudeLaunchObservation::default, || {}, 1, 0);
    let outcome = RECOVER_CALL;
    println!("ready={} process={:?} recovery_calls={} launches={}",
        outcome.ready, outcome.process_id, recoveries.get(), launches.get());
}
'@
$code = "use std::path::PathBuf;`n#[derive(Clone, Debug, PartialEq, Eq)]`n" + $types + $plan + $sanitize + $recovery + $main.Replace('RECOVER_CALL', $recoverCall)
$generated = Join-Path $PSScriptRoot 'launch-fixture.rs'
$exe = Join-Path $PSScriptRoot 'launch-fixture.exe'
[IO.File]::WriteAllText($generated, $code, [Text.UTF8Encoding]::new($false))
& rustc --edition 2024 --cap-lints allow $generated -o $exe
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& $exe
exit $LASTEXITCODE
