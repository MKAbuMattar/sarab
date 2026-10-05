# Sarab gate checks for Windows. Usage: pwsh -NoProfile -File sarab/scripts/check.ps1 <unit|embed|media|picture|pause|budget|prop|restore|kill|ui|switch|explorer|fullscreen|brand|reasons|sync|about|installer|static|preset|updater|launch>
# Each check prints "<name> gate passed" only after every assertion holds, and exits 1 otherwise.
# Sarab runs with APPDATA/LOCALAPPDATA pointed at a sandbox, so real settings are never touched.
param([Parameter(Mandatory)][string]$Gate)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path "$PSScriptRoot/.."
$exe = Join-Path $root 'src-tauri/target/release/sarab.exe'
$fx = Join-Path $PSScriptRoot 'fixtures'
$sandbox = Join-Path $PSScriptRoot '.sandbox'
$cfg = Join-Path $sandbox 'roaming/com.mkabumattar.sarab'
$statusFile = Join-Path $cfg 'status.json'

function Fail($msg) { Write-Host "FAIL: $msg"; Stop-Sarab; exit 1 }
function Assert($cond, $msg) { if (-not $cond) { Fail $msg } }

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class W {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint cmd);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  public static string Text(IntPtr h) { var s = new StringBuilder(256); GetWindowText(h, s, 256); return s.ToString(); }
  public static string Class(IntPtr h) { var s = new StringBuilder(256); GetClassName(h, s, 256); return s.ToString(); }
  public static IntPtr[] Children(IntPtr p) { var l = new System.Collections.Generic.List<IntPtr>(); EnumChildWindows(p, (h, x) => { if (GetParent(h) == p) l.Add(h); return true; }, IntPtr.Zero); return l.ToArray(); }
  public static IntPtr[] TopLevel() { var l = new System.Collections.Generic.List<IntPtr>(); EnumWindows((h, x) => { l.Add(h); return true; }, IntPtr.Zero); return l.ToArray(); }
}
[ComImport, Guid("B92B56A9-8B55-4E14-9A89-0199BBB6F93B"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface IDesktopWallpaper {
  void SetWallpaper([MarshalAs(UnmanagedType.LPWStr)] string monitorID, [MarshalAs(UnmanagedType.LPWStr)] string wallpaper);
  [return: MarshalAs(UnmanagedType.LPWStr)] string GetWallpaper([MarshalAs(UnmanagedType.LPWStr)] string monitorID);
  [return: MarshalAs(UnmanagedType.LPWStr)] string GetMonitorDevicePathAt(uint monitorIndex);
  uint GetMonitorDevicePathCount();
  W.RECT GetMonitorRECT([MarshalAs(UnmanagedType.LPWStr)] string monitorID);
}
// PowerShell sees COM objects through IDispatch, so every call goes through these typed helpers.
public static class DW {
  static IDesktopWallpaper Api() { return (IDesktopWallpaper)Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("C2CF3110-460E-4fc1-B9D0-8A1C0C9CC4BD"))); }
  public static string[] Ids() { var a = Api(); var n = a.GetMonitorDevicePathCount(); var r = new string[n]; for (uint i = 0; i < n; i++) r[i] = a.GetMonitorDevicePathAt(i); return r; }
  public static string Get(string id) { return Api().GetWallpaper(id); }
  public static void Set(string id, string path) { Api().SetWallpaper(id, path); }
  public static W.RECT Rect(string id) { return Api().GetMonitorRECT(id); }
}
'@

# ---- process control ----

function Sarab-Pid { (Get-Process sarab -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe } | Select-Object -First 1).Id }

# Single instance is keyed on the app identifier, so a launch of this build hands its arguments to
# any Sarab already running, including one the user installed. Never let a check drive that copy.
function Assert-NoOtherSarab {
  $other = Get-Process sarab -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path -ne $exe }
  if ($other) { Fail "a Sarab from $($other[0].Path) is running; quit it first, or this check would send its commands to it" }
}

function Start-Sarab([switch]$Fresh) {
  Assert-NoOtherSarab
  if ($Fresh) {
    Stop-Sarab
    if (Test-Path $sandbox) { Remove-Item $sandbox -Recurse -Force }
  }
  New-Item -ItemType Directory -Force "$sandbox/roaming", "$sandbox/local" | Out-Null
  if (-not (Sarab-Pid)) {
    $env:APPDATA = "$sandbox/roaming"; $env:LOCALAPPDATA = "$sandbox/local"
    # --autostart: start in the tray like a login launch, so checks never open the window or turn on Start with Windows.
    Start-Process $exe -ArgumentList '--autostart' | Out-Null
  }
  Wait-For { (Test-Path $statusFile) -and (Status).pid -eq (Sarab-Pid) } 20 'Sarab did not start'
}

function Stop-Sarab {
  $p = Sarab-Pid
  if ($p) {
    $env:APPDATA = "$sandbox/roaming"; $env:LOCALAPPDATA = "$sandbox/local"
    & $exe quit
    $deadline = (Get-Date).AddSeconds(8)
    while ((Sarab-Pid) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 200 }
    if (Sarab-Pid) { Stop-Process -Id (Sarab-Pid) -Force }
  }
}

function Sarab { Assert-NoOtherSarab; $env:APPDATA = "$sandbox/roaming"; $env:LOCALAPPDATA = "$sandbox/local"; & $exe @args; Start-Sleep -Milliseconds 300 }

function Status { try { Get-Content $statusFile -Raw | ConvertFrom-Json } catch { $null } }

function Wait-For([scriptblock]$cond, [double]$seconds, [string]$msg) {
  $deadline = (Get-Date).AddSeconds($seconds)
  while ((Get-Date) -lt $deadline) {
    try { if (& $cond) { return } } catch {}
    Start-Sleep -Milliseconds 150
  }
  Fail $msg
}

function Page([int]$i) {
  # Ask the pages to report, then read the fresh report.
  $before = (Get-Item $statusFile).LastWriteTimeUtc
  Sarab status
  Wait-For { (Get-Item $statusFile).LastWriteTimeUtc -gt $before -and $null -ne (Status).displays[$i].page } 5 "display $i page did not report"
  Start-Sleep -Milliseconds 300
  (Status).displays[$i].page
}

function Tree([int]$rootPid) {
  $all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId
  $ids = @($rootPid); $grew = $true
  while ($grew) {
    $more = $all | Where-Object { $ids -contains $_.ParentProcessId -and $ids -notcontains $_.ProcessId } | ForEach-Object ProcessId
    $grew = [bool]$more; $ids += $more
  }
  $ids
}

# The NSIS installer remembers its last install folder in HKCU\Software\Sarab\Sarab and offers it
# to the next install, even after an uninstall. Test installs must put it back, or the user's own
# install lands in the test folder.
function Save-InstallMemory { $k = 'HKCU:\Software\Sarab\Sarab'; if (Test-Path $k) { (Get-ItemProperty $k).'(default)' } else { $null } }
function Restore-InstallMemory($saved) {
  $k = 'HKCU:\Software\Sarab\Sarab'
  if ($null -ne $saved) { New-Item -Force $k | Out-Null; Set-Item $k $saved }
  elseif (Test-Path $k) { Remove-Item $k -Recurse -Force; if (-not (Get-ChildItem 'HKCU:\Software\Sarab' -ErrorAction SilentlyContinue)) { Remove-Item 'HKCU:\Software\Sarab' -ErrorAction SilentlyContinue } }
}

# ---- fixtures ----

function Ensure-Fixtures {
  $web = Join-Path $fx 'web'
  New-Item -ItemType Directory -Force "$web/js" | Out-Null
  Get-ChildItem $web -File | Where-Object Name -notin 'sarab.json', 'properties.json', 'index.html' | Remove-Item
  Set-Content "$web/sarab.json" '{"title":"Check page","description":"Canvas animation used by the gate checks","author":"sarab","type":"web","file":"index.html","version":1}'
  Set-Content "$web/properties.json" '{"speed":{"type":"slider","value":1,"min":0,"max":5,"step":0.5,"text":"Speed"},"tint":{"type":"color","value":"#D9A36A","text":"Tint"},"hello":{"type":"button","value":"Hello","text":"Hello"}}'
  Set-Content "$web/index.html" @'
<!doctype html><html><head><meta charset="utf-8"><style>html,body{margin:0;height:100%;background:#2B2233}</style></head>
<body><canvas id="c"></canvas><script src="js/app.js"></script></body></html>
'@
  Set-Content "$web/js/app.js" @'
const props = {}; let paused = null;
window.sarabPropertyChanged = (k, v) => { props[k] = v; };
window.sarabPlaybackChanged = s => { paused = s.paused; };
window.sarabStatus = () => ({ appJs: true, props, paused });
const c = document.getElementById('c'), g = c.getContext('2d');
function frame(t) {
  c.width = innerWidth; c.height = innerHeight;
  g.fillStyle = props.tint || '#D9A36A';
  g.fillRect((t * 0.1 * (props.speed || 1)) % innerWidth, innerHeight / 2 - 40, 80, 80);
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
'@
  $ff = (Get-Command ffmpeg -ErrorAction SilentlyContinue).Source
  if (-not (Test-Path "$fx/clip.mp4")) {
    Assert $ff 'ffmpeg is needed once to make the media fixtures'
    & $ff -loglevel error -y -f lavfi -i testsrc2=size=1920x1080:rate=30 -t 10 -c:v libx264 -pix_fmt yuv420p "$fx/clip.mp4"
    & $ff -loglevel error -y -f lavfi -i testsrc=size=320x180:rate=10 -t 3 "$fx/anim.gif"
    & $ff -loglevel error -y -f lavfi -i testsrc2=size=1920x1080 -frames:v 1 "$fx/still.png"
  }
}

function Web-Id { (Status).displays[0].wallpaper }

function Set-Web {
  Sarab set (Join-Path $fx 'web')
  Wait-For { $s = Status; $s.displays.Count -ge 1 -and @($s.displays | Where-Object { -not $_.loaded -or $_.kind -ne 'web' }).Count -eq 0 } 20 'web wallpaper did not load on every display'
}

# ---- gates ----

switch ($Gate) {
  'unit' {
    Push-Location (Join-Path $root 'src-tauri')
    $out = cargo test 2>&1 | Out-String
    Pop-Location
    Write-Host $out
    foreach ($t in 'pause::tests::decide_table', 'library::tests::zip_slip_rejected', 'library::tests::package_zip_round_trip', 'cli::tests::cli_parse', 'library::tests::property_merge_and_coerce', 'library::tests::zip_size_cap') {
      Assert ($out -match [regex]::Escape("test $t ... ok")) "$t did not pass"
    }
    Assert ($out -match 'test result: ok\.') 'cargo test failed'
    'unit gate passed'
  }

  'embed' {
    Ensure-Fixtures; Start-Sarab -Fresh; Set-Web; Sarab play
    $s = Status
    $d = $s.desktop
    Assert $d 'no desktop layer in status'
    $parent = if ($d.raised) { [IntPtr]$d.progman } else { [IntPtr]$d.workerw }
    $kids = [W]::Children($parent)
    for ($i = 0; $i -lt $s.displays.Count; $i++) {
      $h = $kids | Where-Object { [W]::Text($_) -eq "sarab-wp-$i" } | Select-Object -First 1
      Assert $h "sarab-wp-$i is not a child of the desktop layer ($(if ($d.raised) {'Progman'} else {'WorkerW'}))"
      $r = New-Object W+RECT; [void][W]::GetWindowRect($h, [ref]$r)
      $m = $s.displays[$i].rect
      Assert ($r.L -eq $m[0] -and $r.T -eq $m[1] -and $r.R -eq $m[2] -and $r.B -eq $m[3]) "display $i window rect ($($r.L),$($r.T),$($r.R),$($r.B)) != monitor ($($m -join ','))"
      Assert ([W]::IsWindowVisible($h)) "sarab-wp-$i is not visible"
      if ($d.raised) {
        # Z-order among Progman's children: DefView above us, WorkerW below us (last).
        $order = @(); $c = [W]::GetWindow([IntPtr]$d.progman, 5)  # GW_CHILD
        while ($c -ne [IntPtr]::Zero) { $order += $c; $c = [W]::GetWindow($c, 2) }  # GW_HWNDNEXT
        $iDef = [array]::IndexOf($order, [IntPtr]$d.defview); $iMe = [array]::IndexOf($order, $h); $iW = [array]::IndexOf($order, [IntPtr]$d.workerw)
        Assert ($iDef -ge 0 -and $iDef -lt $iMe -and $iMe -lt $iW) "z-order wrong: defview=$iDef wp=$iMe workerw=$iW"
        Assert ($iW -eq $order.Count - 1) 'WorkerW is not the bottom child'
      }
      $p = Page $i
      Assert ($p.page.appJs) "display $i did not run js/app.js (relative link failed)"
      Assert ($p.frames -gt 0) "display $i rendered no frames"
    }
    Write-Host "raised=$($d.raised) displays=$($s.displays.Count)"
    Sarab close; Stop-Sarab
    'embed gate passed'
  }

  'media' {
    Ensure-Fixtures; Start-Sarab -Fresh
    Assert ((Status).displays.Count -ge 2) 'media check needs two displays'
    Sarab set (Join-Path $fx 'clip.mp4') --display 0
    Sarab set (Join-Path $fx 'anim.gif') --display 1
    Sarab play
    Wait-For { $s = Status; $s.displays[0].loaded -and $s.displays[1].loaded } 20 'media did not load'
    $s = Status
    Assert ($s.displays[0].kind -eq 'video' -and $s.displays[1].kind -eq 'gif') "kinds: $($s.displays[0].kind), $($s.displays[1].kind)"
    Start-Sleep 2
    $v = (Page 0).media[0]
    Assert ($v -and -not $v.paused -and $v.time -gt 0.5 -and $v.ready -ge 2) "video not playing: $($v | ConvertTo-Json -Compress)"
    $g = (Page 1).images[0]
    Assert ($g -and $g.complete -and $g.width -eq 320) "gif not decoded: $($g | ConvertTo-Json -Compress)"
    Sarab pause
    Start-Sleep 1
    $t1 = (Page 0).media[0]; Start-Sleep 1; $t2 = (Page 0).media[0]
    Assert ($t1.paused -and $t1.time -eq $t2.time) 'video kept playing while paused'
    Sarab close; Stop-Sarab
    'media gate passed'
  }

  'picture' {
    Ensure-Fixtures
    $ids = [DW]::Ids()
    $orig = @{}; foreach ($id in $ids) { $orig[$id] = [DW]::Get($id) }
    try {
      Start-Sarab -Fresh
      $m0 = (Status).displays[0].rect
      $id0 = $ids | Where-Object { $r = [DW]::Rect($_); $r.L -eq $m0[0] -and $r.T -eq $m0[1] } | Select-Object -First 1
      Assert $id0 'could not match display 0 to a wallpaper monitor id'
      $png = (Resolve-Path "$fx/still.png").Path
      Sarab set $png --display 0
      Wait-For { [DW]::Get($id0) -ieq $png } 10 "OS wallpaper is '$([DW]::Get($id0))', expected $png"
      foreach ($id in $ids | Where-Object { $_ -ne $id0 }) { Assert ([DW]::Get($id) -eq $orig[$id]) 'another display changed' }
      Sarab close --display 0
      Wait-For { [DW]::Get($id0) -eq $orig[$id0] } 10 "close did not restore '$($orig[$id0])'"
      Stop-Sarab
    } finally {
      foreach ($id in $ids) { if ([DW]::Get($id) -ne $orig[$id] -and $orig[$id]) { [DW]::Set($id, $orig[$id]) } }
    }
    Write-Host "original restored: $($orig[$id0])"
    'picture gate passed'
  }

  'pause' {
    Ensure-Fixtures; Start-Sarab -Fresh; Set-Web
    Sarab pause
    Wait-For { @((Status).displays | Where-Object state -ne 'Frozen').Count -eq 0 } 3 'manual pause did not freeze every display'
    $p = Page 0
    Assert ($p.frozen -and $p.page.paused -eq $true) "page not frozen or no pause event: $($p | ConvertTo-Json -Compress)"
    Sarab resume
    Wait-For { (Status).manual -eq $null } 3 'resume did not return to automatic'
    # Negative control: the test display must start uncovered, or "hidden" would prove nothing.
    Start-Sleep 2
    $t = [array]::IndexOf(@((Status).signals.covered), $false)
    Assert ($t -ge 0) 'every display is already covered by a maximized window; restore one and rerun'
    $before = (Status).displays[$t].state
    Assert ($before -ne 'Covered') "display $t is uncovered but already Covered"
    Write-Host "testing display $t (was $before)"
    $m = (Status).displays[$t].rect
    $cover = Start-Process pwsh -PassThru -WindowStyle Normal -ArgumentList '-NoProfile', '-Command', @"
Add-Type -AssemblyName System.Windows.Forms
`$f = New-Object System.Windows.Forms.Form
`$f.Text = 'sarab-cover-test'; `$f.StartPosition = 'Manual'; `$f.Location = New-Object System.Drawing.Point($($m[0] + 50), $($m[1] + 50))
`$f.Add_Shown({ `$f.WindowState = 'Maximized'; `$f.Activate() })
[void]`$f.ShowDialog()
"@
    try {
      Wait-For { [W]::TopLevel() | Where-Object { [W]::Text($_) -eq 'sarab-cover-test' -and [W]::IsWindowVisible($_) } } 15 'cover window did not appear'
      $shown = Get-Date
      Wait-For { (Status).displays[$t].state -eq 'Covered' } 3 'covered display was not paused'
      $lag = ((Get-Date) - $shown).TotalSeconds
      Write-Host ("covered after {0:N2} s" -f $lag)
      Assert ($lag -le 1.5) "pause took $lag s"
      # The wallpaper must stay on screen, frozen: hiding it shows the plain Windows wallpaper.
      $h = [W]::Children([IntPtr](Status).desktop.progman) + [W]::Children([IntPtr](Status).desktop.workerw) | Where-Object { [W]::Text($_) -eq "sarab-wp-$t" } | Select-Object -First 1
      Assert ([W]::IsWindowVisible($h)) 'covered wallpaper window was hidden; the Windows wallpaper would show through'
      Assert ((Page $t).frozen) 'covered wallpaper is not frozen'
    } finally { Stop-Process -Id $cover.Id -Force -ErrorAction SilentlyContinue }
    Wait-For { (Status).displays[$t].state -eq $before } 3 "display $t did not return to $before"
    Sarab close; Stop-Sarab
    'pause gate passed'
  }

  'budget' {
    Ensure-Fixtures; Start-Sarab -Fresh; Set-Web; Sarab play
    Start-Sleep 2
    $tree = Tree (Sarab-Pid)
    function Sum-Cpu { ($tree | ForEach-Object { (Get-Process -Id $_ -ErrorAction SilentlyContinue).TotalProcessorTime.TotalSeconds } | Measure-Object -Sum).Sum }
    $a = Sum-Cpu; Start-Sleep 10; $playing = ((Sum-Cpu) - $a) / 10
    Sarab pause
    Start-Sleep 3
    $tree = Tree (Sarab-Pid)
    $a = Sum-Cpu; Start-Sleep 20; $paused = ((Sum-Cpu) - $a) / 20
    $ram = ($tree | ForEach-Object { (Get-Process -Id $_ -ErrorAction SilentlyContinue).PrivateMemorySize64 } | Measure-Object -Sum).Sum / 1MB
    Write-Host ("processes={0} playing={1:N4} cores paused={2:N4} cores private RAM={3:N0} MB" -f $tree.Count, $playing, $paused, $ram)
    Assert ($playing -gt $paused) 'measurement is not sensitive: playing did not cost more than paused'
    Assert ($paused -lt 0.02) "paused CPU $paused cores is over 0.02"
    Sarab close; Stop-Sarab
    'budget gate passed'
  }

  'prop' {
    Ensure-Fixtures; Start-Sarab -Fresh; Set-Web; Sarab play
    Sarab prop speed=3
    $id = Web-Id
    foreach ($i in 0..((Status).displays.Count - 1)) {
      $key = ((Status).displays[$i].key -replace '[^A-Za-z0-9]', '')
      $file = Join-Path $cfg "props/$id/$key.json"
      Wait-For { (Get-Content $file -Raw | ConvertFrom-Json).speed -eq 3 } 5 "saved props for display $i missing speed=3 ($file)"
      $p = Page $i
      Assert ($p.page.props.speed -eq 3) "page on display $i got speed $($p.page.props.speed)"
      Assert ($p.page.props.tint -eq '#D9A36A') 'initial properties were not sent on load'
    }
    Sarab prop speed=abc
    Assert ((Get-Content (Join-Path $cfg 'sarab.log') -Raw) -match 'not a number') 'bad value was not rejected'
    Sarab volume 40
    Wait-For { (Get-Content (Join-Path $cfg 'settings.json') -Raw | ConvertFrom-Json).volume -eq 40 -and (Status).volume -eq 40 } 5 'volume not saved'
    Sarab close; Stop-Sarab
    'prop gate passed'
  }

  'restore' {
    Ensure-Fixtures; Start-Sarab -Fresh; Set-Web
    $id = Web-Id; $n = (Status).displays.Count
    Stop-Sarab
    Assert (-not (Sarab-Pid)) 'quit did not exit'
    Start-Sarab
    Wait-For { $s = Status; @($s.displays | Where-Object { $_.wallpaper -eq $id -and $_.loaded }).Count -eq $n } 20 'layout was not restored on every display'
    Sarab close; Stop-Sarab
    'restore gate passed'
  }

  'kill' {
    Ensure-Fixtures; Start-Sarab -Fresh; Set-Web
    $tree = Tree (Sarab-Pid)
    Assert ($tree.Count -gt 2) "expected WebView2 child processes, found $($tree.Count - 1)"
    Stop-Process -Id (Sarab-Pid) -Force
    Start-Sleep 5
    $alive = @($tree | Where-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue })
    Assert ($alive.Count -eq 0) "left behind: $($alive -join ', ')"
    Write-Host "children cleaned up: $($tree.Count - 1)"
    'kill gate passed'
  }

  'ui' {
    $en = Get-Content (Join-Path $root 'ui/i18n/en.json') -Raw | ConvertFrom-Json -AsHashtable
    $ar = Get-Content (Join-Path $root 'ui/i18n/ar.json') -Raw | ConvertFrom-Json -AsHashtable
    $missing = @($en.Keys | Where-Object { -not $ar.ContainsKey($_) }) + @($ar.Keys | Where-Object { -not $en.ContainsKey($_) })
    Assert ($missing.Count -eq 0) "string keys differ: $($missing -join ', ')"
    $used = Select-String -Path (Join-Path $root 'ui/*.html'), (Join-Path $root 'ui/app.js') -Pattern "(data-t[pl]?=""|t\(')([a-zA-Z.]+)" -AllMatches | ForEach-Object { $_.Matches } | ForEach-Object { $_.Groups[2].Value } | Sort-Object -Unique
    $unknown = @($used | Where-Object { -not $en.ContainsKey($_) })
    Assert ($unknown.Count -eq 0) "UI uses keys with no string: $($unknown -join ', ')"
    Assert ((Get-Content (Join-Path $root 'ui/app.js') -Raw) -match "lang === 'ar' \? 'rtl'") 'Arabic does not switch to rtl'
    $css = Get-Content (Join-Path $root 'ui/style.css') -Raw
    foreach ($hex in '#EAE6DB', '#D9C9B0', '#2B2233', '#3C3C3C', '#2F5D6B', '#8B1E2D', '#6B7A4F', '#D9A36A', '#C76B6B', '#B9875E') {
      Assert ($css -match $hex) "style.css lacks token $hex"
    }
    Assert ($css -match 'data-theme=dark') 'no dark theme'
    Assert ($css -match 'html\.translucent') 'no translucent background rule'
    Assert ($css -match 'select\.native') 'native dropdown popups are not replaced'
    Assert ((Get-Content (Join-Path $root 'ui/index.html') -Raw) -match 'name="theme"') 'no theme setting'
    Assert ((Get-Content (Join-Path $root 'src-tauri/src/main.rs') -Raw) -match 'Effect::Mica') 'Mica is not requested'
    Start-Sarab -Fresh
    Sarab ui
    $pidS = Sarab-Pid
    Wait-For { [W]::TopLevel() | Where-Object { $o = 0; [void][W]::GetWindowThreadProcessId($_, [ref]$o); $o -eq $pidS -and [W]::Text($_) -eq 'Sarab' -and [W]::IsWindowVisible($_) } } 15 'settings window did not open'
    Stop-Sarab
    'ui gate passed'
  }

  'switch' {
    Ensure-Fixtures; Start-Sarab -Fresh; Set-Web; Sarab play
    $webId = Web-Id
    function Expect-On0($kind) {
      Wait-For { $d = (Status).displays[0]; $d.kind -eq $kind -and $d.loaded -and -not $d.error } 20 "display 0 did not switch to $kind cleanly: $((Status).displays[0] | ConvertTo-Json -Compress)"
      Start-Sleep 1
      $d = (Status).desktop
      $all = [W]::Children([IntPtr]$d.progman) + [W]::Children([IntPtr]$d.workerw)
      for ($i = 0; $i -lt (Status).displays.Count; $i++) {
        $n = @($all | Where-Object { [W]::Text($_) -eq "sarab-wp-$i" }).Count
        Assert ($n -eq 1) "display $i has $n wallpaper windows after switching to $kind"
      }
    }
    Sarab set (Join-Path $fx 'clip.mp4') --display 0
    Expect-On0 'video'
    Sarab set $webId --display 0
    Expect-On0 'web'
    Sarab set (Join-Path $fx 'anim.gif') --display 0
    Expect-On0 'gif'
    Assert ((Status).displays[1].wallpaper -eq $webId) 'display 1 changed while switching display 0'
    Sarab close; Stop-Sarab
    'switch gate passed'
  }

  'explorer' {
    # Restarts explorer.exe: the taskbar blinks and File Explorer windows close. Run only with consent.
    Ensure-Fixtures; Start-Sarab -Fresh; Set-Web; Sarab play
    $oldProgman = (Status).desktop.progman
    Get-Process explorer | Stop-Process -Force
    Start-Sleep 2
    if (-not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process explorer }
    $t0 = Get-Date
    Wait-For { $p = [W]::TopLevel() | Where-Object { [W]::Class($_) -eq 'Progman' }; $p -and [long]$p[0] -ne $oldProgman } 30 'Explorer did not come back'
    $shell = Get-Date
    Wait-For {
      $s = Status
      $s.desktop.progman -ne $oldProgman -and @($s.displays | Where-Object { -not $_.loaded -or $_.error }).Count -eq 0
    } 15 "wallpapers did not re-attach: $((Status) | ConvertTo-Json -Depth 4 -Compress)"
    $lag = ((Get-Date) - $shell).TotalSeconds
    $s = Status; $d = $s.desktop
    $parent = if ($d.raised) { [IntPtr]$d.progman } else { [IntPtr]$d.workerw }
    $kids = [W]::Children($parent)
    for ($i = 0; $i -lt $s.displays.Count; $i++) {
      $h = $kids | Where-Object { [W]::Text($_) -eq "sarab-wp-$i" } | Select-Object -First 1
      Assert $h "sarab-wp-$i is not under the new desktop layer"
      if ($d.raised) {
        $order = @(); $c = [W]::GetWindow([IntPtr]$d.progman, 5)
        while ($c -ne [IntPtr]::Zero) { $order += $c; $c = [W]::GetWindow($c, 2) }
        $iDef = [array]::IndexOf($order, [IntPtr]$d.defview); $iMe = [array]::IndexOf($order, $h); $iW = [array]::IndexOf($order, [IntPtr]$d.workerw)
        Assert ($iDef -ge 0 -and $iDef -lt $iMe -and $iMe -lt $iW) "z-order wrong after restart: defview=$iDef wp=$iMe workerw=$iW"
      }
    }
    Write-Host ("re-attached {0:N1} s after the new shell appeared" -f $lag)
    Assert ($lag -le 5) "re-attach took $lag s"
    Sarab close; Stop-Sarab
    'explorer gate passed'
  }

  'fullscreen' {
    # A real fullscreen player, then a deliberate re-stack: the wallpaper must never drop below WorkerW.
    Ensure-Fixtures; Start-Sarab -Fresh; Set-Web
    $ffplay = Join-Path (Split-Path (Get-Command ffmpeg).Source) 'ffplay.exe'
    Assert (Test-Path $ffplay) 'ffplay is needed for the fullscreen check'
    $d = (Status).desktop
    Assert $d.raised 'this check is written for the raised desktop (Windows 11 24H2+)'
    function Order { $o = @(); $c = [W]::GetWindow([IntPtr]$d.progman, 5); while ($c -ne [IntPtr]::Zero) { $o += $c; $c = [W]::GetWindow($c, 2) }; $o }
    function Assert-Order($when) {
      $o = Order
      $iDef = [array]::IndexOf($o, [IntPtr]$d.defview); $iW = [array]::IndexOf($o, [IntPtr]$d.workerw)
      foreach ($i in 0..((Status).displays.Count - 1)) {
        $h = $o | Where-Object { [W]::Text($_) -eq "sarab-wp-$i" } | Select-Object -First 1
        $iMe = [array]::IndexOf($o, $h)
        Assert ($h -and $iDef -lt $iMe -and $iMe -lt $iW -and [W]::IsWindowVisible($h)) "$when : wallpaper $i not between DefView and WorkerW (def=$iDef wp=$iMe workerw=$iW)"
      }
    }
    $p = Start-Process $ffplay -PassThru -ArgumentList '-fs', '-loglevel', 'quiet', '-loop', '0', (Join-Path $fx 'clip.mp4')
    try {
      $end = (Get-Date).AddSeconds(5); $n = 0
      while ((Get-Date) -lt $end) { Assert-Order 'during fullscreen'; $n++; Start-Sleep -Milliseconds 200 }
      Assert (@((Status).displays | Where-Object { $_.reason -eq 'covered' }).Count -ge 1) 'no display reports reason covered while a fullscreen player runs'
    } finally { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
    $end = (Get-Date).AddSeconds(3)
    while ((Get-Date) -lt $end) { Assert-Order 'after fullscreen'; $n++; Start-Sleep -Milliseconds 200 }
    Write-Host "z-order held over $n samples"
    # Negative control: put WorkerW on top, as a misbehaving shell would. The guard must undo it.
    Add-Type -Namespace Z -Name U -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);'
    [void][Z.U]::SetWindowPos([IntPtr]$d.workerw, [IntPtr]::Zero, 0, 0, 0, 0, 0x13)   # HWND_TOP, NOMOVE|NOSIZE|NOACTIVATE
    $broken = [array]::IndexOf((Order), [IntPtr]$d.workerw)
    Assert ($broken -eq 0) "could not break the order for the control (workerw at $broken)"
    Wait-For { $o = Order; [array]::IndexOf($o, [IntPtr]$d.workerw) -eq $o.Count - 1 } 3 'guard did not move WorkerW back to the bottom'
    Assert-Order 'after repair'
    Assert ((Get-Content (Join-Path $cfg 'sarab.log') -Raw) -match 'z-order repaired') 'repair was not logged'
    Sarab close; Stop-Sarab
    'fullscreen gate passed'
  }

  'brand' {
    # No trace of the old format or API anywhere in the app's own files.
    # The word is assembled so this script does not match itself.
    $word = -join ('l', 'i', 'v', 'e', 'l', 'y')
    $skip = '[\\/](target|gen|\.sandbox|fixtures)[\\/]'
    function Hits($dir) { Get-ChildItem $dir -Recurse -File | Where-Object { $_.FullName -notmatch $skip -and $_.Extension -notin '.png', '.ico' } | Select-String -Pattern $word -List | ForEach-Object { $_.Path } }
    # Positive control: the scan must find a planted hit.
    $probe = Join-Path $env:TEMP 'sarab-brand-probe'; New-Item -ItemType Directory -Force $probe | Out-Null
    Set-Content "$probe/x.txt" "an old $($word.ToUpper()) manifest"
    Assert (@(Hits $probe).Count -eq 1) 'scan cannot find a planted hit'
    Remove-Item $probe -Recurse -Force
    $found = @(Hits $root)
    Assert ($found.Count -eq 0) "old app name still in: $($found -join ', ')"
    $n = @(Get-ChildItem $root -Recurse -File | Where-Object { $_.FullName -notmatch $skip }).Count
    Write-Host "scanned $n files"
    'brand gate passed'
  }

  'reasons' {
    # Every pause reason the core can send has words in both languages.
    $src = Get-Content (Join-Path $root 'src-tauri/src/pause.rs') -Raw
    $variants = ([regex]::Match($src, 'pub enum Reason \{([^}]*)\}').Groups[1].Value -split ',') | ForEach-Object { $_.Trim() } | Where-Object { $_ }
    Assert ($variants.Count -ge 10) "could not read the Reason enum ($($variants.Count))"
    $snake = $variants | ForEach-Object { ($_ -creplace '([a-z])([A-Z])', '$1_$2').ToLower() }
    foreach ($lang in 'en', 'ar') {
      $j = Get-Content (Join-Path $root "ui/i18n/$lang.json") -Raw | ConvertFrom-Json -AsHashtable
      $miss = @($snake | Where-Object { -not $j.ContainsKey("reason.$_") })
      Assert ($miss.Count -eq 0) "$lang has no words for: $($miss -join ', ')"
    }
    # Live: the core reports the reason that actually holds the pause.
    Ensure-Fixtures; Start-Sarab -Fresh; Set-Web
    Sarab pause
    Wait-For { @((Status).displays | Where-Object reason -ne 'manual').Count -eq 0 } 3 'manual pause did not report reason manual'
    Sarab play
    Wait-For { @((Status).displays | Where-Object { $_.reason -ne 'forced_play' -or $_.state -ne 'Play' }).Count -eq 0 } 3 'play anyway did not report forced_play'
    Sarab resume
    Wait-For { $s = Status; @($s.displays | Where-Object { ($_.state -eq 'Play') -ne ($_.reason -in 'none', 'app_play') }).Count -eq 0 } 3 "automatic state and reason disagree: $((Status).displays | ConvertTo-Json -Compress)"
    Write-Host "reasons: $($snake -join ' ')"
    Sarab close; Stop-Sarab
    'reasons gate passed'
  }

  'sync' {
    # The same video on two displays plays in step, including after one display restarts alone.
    Ensure-Fixtures; Start-Sarab -Fresh
    Assert ((Status).displays.Count -ge 2) 'sync needs two displays'
    Sarab set (Join-Path $fx 'clip.mp4'); Sarab play
    Wait-For { @((Status).displays | Where-Object { -not $_.loaded -or $_.kind -ne 'video' }).Count -eq 0 } 20 'video did not load on every display'
    # Position on a shared clock: media time minus wall clock, compared around the 10 s loop.
    function Pos($i) { $p = Page $i; $p.media[0].time - $p.clock / 1000 }
    function Gap { $x = ((((Pos 0) - (Pos 1)) % 10) + 15) % 10 - 5; [math]::Abs($x) }
    Start-Sleep 6
    $g1 = Gap; Write-Host ("gap after start: {0:N3} s" -f $g1)
    Assert ($g1 -lt 0.1) "displays are $g1 s apart after start"
    # Control: display 1 restarts at 0 while display 0 is mid-clip, so unsynced they would differ by that much.
    $lead = (Page 0).media[0].time
    Sarab close --display 1
    Sarab set (Join-Path $fx 'clip.mp4') --display 1
    Wait-For { $d = (Status).displays[1]; $d.loaded -and $d.kind -eq 'video' } 20 'display 1 did not reload'
    Start-Sleep 7
    $g2 = Gap; Write-Host ("display 0 was at {0:N2} s when display 1 restarted; gap now {1:N3} s" -f $lead, $g2)
    Assert ($g2 -lt 0.1) "displays are $g2 s apart after display 1 restarted"
    # And after a pause and resume.
    Sarab pause; Start-Sleep 3; Sarab play; Start-Sleep 6
    $g3 = Gap; Write-Host ("gap after pause and resume: {0:N3} s" -f $g3)
    Assert ($g3 -lt 0.1) "displays are $g3 s apart after pause and resume"
    Sarab close; Stop-Sarab
    'sync gate passed'
  }

  'about' {
    $want = '0.0.3'
    $cargo = [regex]::Match((Get-Content (Join-Path $root 'src-tauri/Cargo.toml') -Raw), '(?m)^version = "([^"]+)"').Groups[1].Value
    $conf = (Get-Content (Join-Path $root 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
    $file = (Get-Item $exe).VersionInfo.ProductVersion
    Write-Host "Cargo.toml=$cargo tauri.conf.json=$conf sarab.exe=$file"
    Assert ($cargo -eq $want -and $conf -eq $want) "source version is $cargo / $conf, expected $want"
    Assert ($file -like "$want*") "sarab.exe reports $file"
    Assert (Test-Path (Join-Path $root "src-tauri/target/release/bundle/nsis/Sarab_${want}_x64-setup.exe")) 'no installer for this version'
    $html = Get-Content (Join-Path $root 'ui/index.html') -Raw
    Assert ($html -match 'id="page-about"' -and $html -match 'data-page="about"') 'About page or its nav item is missing'
    Assert ((Get-Content (Join-Path $root 'ui/app.js') -Raw) -match "about-version") 'About does not show the version'
    'about gate passed'
  }

  'installer' {
    # Silent install into a temp folder, run it, silent uninstall. The user's data must survive.
    $setup = Get-ChildItem (Join-Path $root 'src-tauri/target/release/bundle/nsis') -Filter 'Sarab_*_x64-setup.exe' | Sort-Object LastWriteTime | Select-Object -Last 1
    Assert $setup 'no installer built'
    $bytes = [IO.File]::ReadAllBytes($setup.FullName)
    Assert ([Text.Encoding]::ASCII.GetString($bytes, 0, 2) -eq 'MZ' -and [Text.Encoding]::ASCII.GetString($bytes) -match 'Nullsoft') 'installer is not an NSIS executable'
    Assert (-not (Get-Process sarab -ErrorAction SilentlyContinue)) 'a Sarab is running; the installer would close it'
    # The test install shares the "Sarab" Apps entry and Start with Windows value with a real install.
    Assert (-not (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarab')) 'Sarab is installed for real on this PC; uninstall it before this check'
    $userData = @("$env:APPDATA\com.mkabumattar.sarab", "$env:LOCALAPPDATA\com.mkabumattar.sarab\Library") | Where-Object { Test-Path $_ }
    $before = @($userData | ForEach-Object { Get-ChildItem $_ -Recurse -File -Force } ).Count
    # Long path on purpose: $env:TEMP is 8.3 (MKABUM~1), and the uninstaller only removes shortcuts
    # whose target matches its install folder as written, so a short path leaves them behind.
    $dir = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Temp\sarab-install-check'
    if (Test-Path $dir) { Remove-Item $dir -Recurse -Force }
    $key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarab'
    $menu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Sarab'
    $memory = Save-InstallMemory
    try {
      $p = Start-Process $setup.FullName -ArgumentList '/S', "/D=$dir" -PassThru -Wait
      Assert ($p.ExitCode -eq 0) "installer exit code $($p.ExitCode)"
      Assert (Test-Path "$dir\sarab.exe") 'sarab.exe was not installed'
      Assert (Test-Path "$dir\uninstall.exe") 'no uninstaller'
      Assert (Test-Path $key) 'no uninstall entry in Apps'
      $reg = Get-ItemProperty $key
      Write-Host "installed $($reg.DisplayName) $($reg.DisplayVersion) by $($reg.Publisher) to $dir"
      Assert ($reg.DisplayVersion -eq '0.0.3') "Apps lists version $($reg.DisplayVersion)"
      Assert (Get-ChildItem $menu, (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs') -Filter 'Sarab*.lnk' -ErrorAction SilentlyContinue) 'no Start menu shortcut'
      # The installed app starts (sandboxed so it does not touch the user's settings).
      $env:APPDATA = "$sandbox/roaming"; $env:LOCALAPPDATA = "$sandbox/local"
      if (Test-Path $sandbox) { Remove-Item $sandbox -Recurse -Force }
      New-Item -ItemType Directory -Force "$sandbox/roaming", "$sandbox/local" | Out-Null
      $app = Start-Process "$dir\sarab.exe" -ArgumentList '--autostart' -PassThru
      Wait-For { (Test-Path $statusFile) -and (Status).pid -eq $app.Id } 20 'installed Sarab did not start'
      & "$dir\sarab.exe" quit; Start-Sleep 3
      Assert ($app.HasExited) 'installed Sarab did not quit'
      # A plain launch turns on Start with Windows; the uninstaller must remove it again.
      $app = Start-Process "$dir\sarab.exe" -PassThru
      Wait-For { (Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue).Sarab -like "*sarab-install-check*" } 20 'installed Sarab did not turn on Start with Windows'
      & "$dir\sarab.exe" quit; Start-Sleep 3
      $env:APPDATA = [Environment]::GetFolderPath('ApplicationData'); $env:LOCALAPPDATA = [Environment]::GetFolderPath('LocalApplicationData')
    } finally {
      $env:APPDATA = [Environment]::GetFolderPath('ApplicationData'); $env:LOCALAPPDATA = [Environment]::GetFolderPath('LocalApplicationData')
      if (Test-Path "$dir\uninstall.exe") { Start-Process "$dir\uninstall.exe" -ArgumentList '/S' -Wait | Out-Null; Start-Sleep 3 }
      Restore-InstallMemory $memory
    }
    Assert (-not (Test-Path "$dir\sarab.exe")) 'uninstall left sarab.exe behind'
    Assert (-not (Test-Path $key)) 'uninstall left the Apps entry'
    Assert (-not (Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue).Sarab) 'uninstall left the Start with Windows entry'
    $wsh = New-Object -ComObject WScript.Shell
    $left = @([Environment]::GetFolderPath('Desktop'), "$env:APPDATA\Microsoft\Windows\Start Menu\Programs") | ForEach-Object { Get-ChildItem $_ -Recurse -Filter *.lnk -ErrorAction SilentlyContinue } | Where-Object { $wsh.CreateShortcut($_.FullName).TargetPath -like '*\sarab-install-check\*' }
    Assert (-not $left) "shortcuts left behind: $($left.FullName -join ', ')"
    $after = @($userData | ForEach-Object { Get-ChildItem $_ -Recurse -File -Force -ErrorAction SilentlyContinue }).Count
    Write-Host "user data files before=$before after=$after"
    Assert ($after -eq $before) 'uninstall touched the user data folders'
    'installer gate passed'
  }

  'static' {
    # Everything that can be checked without a desktop, so CI can run it.
    $i18n = Join-Path $root 'ui/i18n'
    $files = Get-ChildItem $i18n -Filter '*.json'
    Assert ($files.Count -ge 2) 'expected at least English and Arabic'
    # Every language in the menu has a file, and every file is in the menu.
    $html = Get-Content (Join-Path $root 'ui/index.html') -Raw
    $menu = [regex]::Match($html, '<select name="language">(.*?)</select>').Groups[1].Value
    $offered = @([regex]::Matches($menu, 'value="([\w-]+)"') | ForEach-Object { $_.Groups[1].Value }) | Sort-Object
    $have = @($files | ForEach-Object { $_.BaseName }) | Sort-Object
    Assert (-not (Compare-Object $offered $have)) "language menu ($($offered -join ',')) and ui/i18n ($($have -join ',')) differ"
    $en = Get-Content (Join-Path $i18n 'en.json') -Raw | ConvertFrom-Json -AsHashtable
    foreach ($f in $files) {
      $j = Get-Content $f.FullName -Raw | ConvertFrom-Json -AsHashtable
      $diff = @($en.Keys | Where-Object { -not $j.ContainsKey($_) }) + @($j.Keys | Where-Object { -not $en.ContainsKey($_) })
      Assert ($diff.Count -eq 0) "$($f.Name) keys differ from en.json: $($diff -join ', ')"
      $empty = @($j.Keys | Where-Object { -not "$($j[$_])".Trim() })
      Assert ($empty.Count -eq 0) "$($f.Name) has empty strings: $($empty -join ', ')"
    }
    $used = Select-String -Path (Join-Path $root 'ui/*.html'), (Join-Path $root 'ui/app.js') -Pattern "(data-t[pl]?=""|\bt\(')([a-zA-Z.]+)" -AllMatches | ForEach-Object { $_.Matches } | ForEach-Object { $_.Groups[2].Value } | Sort-Object -Unique
    $unknown = @($used | Where-Object { -not $en.ContainsKey($_) })
    Assert ($unknown.Count -eq 0) "UI uses keys with no string: $($unknown -join ', ')"
    $src = Get-Content (Join-Path $root 'src-tauri/src/pause.rs') -Raw
    $reasons = ([regex]::Match($src, 'pub enum Reason \{([^}]*)\}').Groups[1].Value -split ',') | ForEach-Object { $_.Trim() } | Where-Object { $_ } | ForEach-Object { ($_ -creplace '([a-z])([A-Z])', '$1_$2').ToLower() }
    $noWords = @($reasons | Where-Object { -not $en.ContainsKey("reason.$_") })
    Assert ($reasons.Count -ge 10 -and $noWords.Count -eq 0) "pause reasons without words: $($noWords -join ', ')"
    # The browser's own dialogs say "tauri.localhost says" and ignore the theme; ask() in app.js replaces them.
    $native = Select-String -Path (Join-Path $root 'ui/*.js'), (Join-Path $root 'ui/*.html') -Pattern '(?<![\w.])(confirm|alert|prompt)\s*\(' | Where-Object { $_.Line -notmatch '^\s*//' }
    Assert (-not $native) "native browser dialog used: $($native | ForEach-Object { '{0}:{1}' -f $_.Filename, $_.LineNumber })"
    $cargo = [regex]::Match((Get-Content (Join-Path $root 'src-tauri/Cargo.toml') -Raw), '(?m)^version = "([^"]+)"').Groups[1].Value
    $conf = (Get-Content (Join-Path $root 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
    Assert ($cargo -eq $conf) "Cargo.toml says $cargo, tauri.conf.json says $conf"
    Assert ((Get-Content (Join-Path $root 'CHANGELOG.md') -Raw) -match "## \[$([regex]::Escape($cargo))\]") "CHANGELOG.md has no entry for $cargo"
    # Library filter and sort (ui/filter.js) behave as tested.
    $filters = & node (Join-Path $root 'scripts/check_filters.mjs') 2>&1
    Assert ($LASTEXITCODE -eq 0) "library filters: $filters"
    Write-Host "$($files.Count) languages, $($en.Count) strings, $($used.Count) keys used, $($reasons.Count) reasons, version $cargo"
    'static gate passed'
  }

  'preset' {
    # Real download from NASA, pinned checksum, then playback inside the clip range.
    $id = 'nasa-iss-earth-view-4k'
    $cat = Get-Content (Join-Path $root 'src-tauri/src/presets.json') -Raw | ConvertFrom-Json
    $p = $cat | Where-Object id -eq $id
    Assert $p "no preset $id in the catalog"
    Start-Sarab -Fresh
    $dir = Join-Path $sandbox "local/com.mkabumattar.sarab/Library/$id"
    $t0 = Get-Date
    Sarab preset $id
    Wait-For { Test-Path "$dir/sarab.json" } 600 "preset did not finish downloading (see sarab.log)"
    $secs = ((Get-Date) - $t0).TotalSeconds
    $file = Join-Path $dir "$id.mp4"
    $hash = (Get-FileHash $file -Algorithm SHA256).Hash.ToLower()
    Assert ($hash -eq $p.sha256) "installed file hash $hash does not match the catalog"
    Assert (-not (Get-ChildItem $dir -Filter '*.part')) 'a partial file was left behind'
    $m = Get-Content "$dir/sarab.json" -Raw | ConvertFrom-Json
    Assert ($m.type -eq 'video' -and $m.clip[0] -eq 6 -and $m.clip[1] -eq 81) "manifest: $($m | ConvertTo-Json -Compress)"
    Write-Host ("downloaded {0:N0} MB in {1:N0} s, sha256 ok" -f ($p.size / 1MB), $secs)
    Sarab set $id --display 0
    Sarab play
    Wait-For { $d = (Status).displays[0]; $d.loaded -and $d.wallpaper -eq $id } 30 'preset did not load'
    Start-Sleep 3
    $v = (Page 0).media[0]
    Write-Host ("playing at {0:N1} s" -f $v.time)
    Assert ($v -and -not $v.paused -and $v.time -ge 6 -and $v.time -lt 81) "not playing inside the 6 to 81 s clip: $($v | ConvertTo-Json -Compress)"
    Sarab close; Stop-Sarab
    'preset gate passed'
  }

  'updater' {
    # End to end: a test 0.0.3 finds a signed 0.0.4 on a local server, downloads, verifies and installs it.
    # Needs the signing key (TAURI_SIGNING_PRIVATE_KEY and _PASSWORD), the same one release.yml uses.
    Assert $env:TAURI_SIGNING_PRIVATE_KEY 'set TAURI_SIGNING_PRIVATE_KEY (and _PASSWORD) to the updater signing key'
    Assert (-not (Get-Process sarab -ErrorAction SilentlyContinue)) 'a Sarab is running; the installer would close it'
    # The test install shares the "Sarab" Apps entry and Start with Windows value with a real install.
    Assert (-not (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarab')) 'Sarab is installed for real on this PC; uninstall it before this check'
    $port = 8765
    # Long path on purpose; see the installer check.
    $work = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Temp\sarab-update-check'
    if (Test-Path $work) { Remove-Item $work -Recurse -Force }
    New-Item -ItemType Directory -Force "$work/feed" | Out-Null
    $env:CARGO_TARGET_DIR = Join-Path $root 'src-tauri/target-updatetest'
    $bundle = Join-Path $env:CARGO_TARGET_DIR 'release/bundle/nsis'
    Push-Location (Join-Path $root 'src-tauri')
    try {
      # 0.0.4: the update that the feed offers.
      '{"version":"0.0.4"}' | Set-Content "$work/new.json"
      cargo tauri build --config "$work/new.json" 2>&1 | Out-Null
      Assert ($LASTEXITCODE -eq 0) 'building 0.0.4 failed'
      Copy-Item "$bundle/Sarab_0.0.4_x64-setup.exe", "$bundle/Sarab_0.0.4_x64-setup.exe.sig" "$work/feed/"
      # Test 0.0.3: same app, but its feed is the local server. Only this build may use plain HTTP.
      @{ version = '0.0.3'; plugins = @{ updater = @{ endpoints = @("http://127.0.0.1:$port/latest.json"); dangerousInsecureTransportProtocol = $true } } } | ConvertTo-Json -Depth 5 | Set-Content "$work/old.json"
      cargo tauri build --config "$work/old.json" 2>&1 | Out-Null
      Assert ($LASTEXITCODE -eq 0) 'building the test 0.0.3 failed'
      Copy-Item "$bundle/Sarab_0.0.3_x64-setup.exe" "$work/old-setup.exe"
    } finally { Pop-Location; Remove-Item Env:CARGO_TARGET_DIR }
    @{ version = '0.0.4'; notes = 'Test release'; pub_date = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
       platforms = @{ 'windows-x86_64' = @{ signature = (Get-Content "$work/feed/Sarab_0.0.4_x64-setup.exe.sig" -Raw).Trim(); url = "http://127.0.0.1:$port/Sarab_0.0.4_x64-setup.exe" } } } |
      ConvertTo-Json -Depth 5 | Set-Content "$work/feed/latest.json"
    $server = Start-Process python -ArgumentList '-m', 'http.server', $port, '--bind', '127.0.0.1', '--directory', "$work/feed" -PassThru -WindowStyle Hidden
    $dir = Join-Path $work 'app'   # NSIS /D= needs backslashes
    $key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarab'
    $memory = Save-InstallMemory
    try {
      Start-Sleep 2
      $p = Start-Process (Join-Path $work 'old-setup.exe') -ArgumentList '/S', "/D=$dir" -PassThru -Wait
      Assert ($p.ExitCode -eq 0 -and (Get-ItemProperty $key).DisplayVersion -eq '0.0.3') 'test 0.0.3 did not install'
      $env:APPDATA = "$sandbox/roaming"; $env:LOCALAPPDATA = "$sandbox/local"
      if (Test-Path $sandbox) { Remove-Item $sandbox -Recurse -Force }
      New-Item -ItemType Directory -Force "$sandbox/roaming", "$sandbox/local" | Out-Null
      Start-Process "$dir/sarab.exe" -ArgumentList '--autostart' | Out-Null
      Wait-For { (Test-Path $statusFile) } 20 'installed 0.0.3 did not start'
      & "$dir/sarab.exe" check-update
      Wait-For { (Get-Content (Join-Path $cfg 'sarab.log') -Raw) -match 'update available: 0\.0\.4' } 30 "0.0.3 did not find 0.0.4: $((Get-Content (Join-Path $cfg 'sarab.log') -Tail 5) -join ' | ')"
      & "$dir/sarab.exe" install-update
      Wait-For { (Get-ItemProperty $key -ErrorAction SilentlyContinue).DisplayVersion -eq '0.0.4' } 180 "update did not install: $((Get-Content (Join-Path $cfg 'sarab.log') -Tail 5) -join ' | ')"
      Wait-For { (Get-Item "$dir/sarab.exe").VersionInfo.ProductVersion -like '0.0.4*' } 30 'installed exe is not 0.0.4'
      Write-Host "updated in place: $((Get-ItemProperty $key).DisplayVersion) at $dir"
      # The update installer keeps running after the version flips (it recreates shortcuts and relaunches
      # Sarab). Uninstalling before it exits leaves shortcuts pointing at a deleted exe.
      Wait-For { -not (Get-Process | Where-Object { $_.Path -like "$work*" -and $_.Name -ne 'sarab' }) } 60 'update installer did not finish'
      $log = Get-Content (Join-Path $cfg 'sarab.log') -Raw
      Assert ($log -notmatch 'update notification:') "the update notification failed: $(($log -split "`n" | Select-String 'update notification') -join ' ')"
    } finally {
      Get-Process sarab -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$work*" } | Stop-Process -Force
      $env:APPDATA = [Environment]::GetFolderPath('ApplicationData'); $env:LOCALAPPDATA = [Environment]::GetFolderPath('LocalApplicationData')
      Start-Sleep 2
      if (Test-Path "$dir/uninstall.exe") { Start-Process "$dir/uninstall.exe" -ArgumentList '/S' -Wait | Out-Null; Start-Sleep 3 }
      Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
      Restore-InstallMemory $memory
    }
    Assert (-not (Test-Path $key)) 'uninstall left the Apps entry'
    $wsh = New-Object -ComObject WScript.Shell
    $left = @([Environment]::GetFolderPath('Desktop'), "$env:APPDATA\Microsoft\Windows\Start Menu\Programs") | ForEach-Object { Get-ChildItem $_ -Recurse -Filter *.lnk -ErrorAction SilentlyContinue } | Where-Object { $wsh.CreateShortcut($_.FullName).TargetPath -like '*\sarab-update-check\*' }
    Assert (-not $left) "shortcuts left behind: $($left.FullName -join ', ')"
    'updater gate passed'
  }

  'launch' {
    # A plain launch opens the window and turns on Start with Windows once; a login launch stays in the tray.
    $run = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
    $before = (Get-ItemProperty $run -ErrorAction SilentlyContinue).Sarab
    Assert (-not $before) "a Start with Windows entry for Sarab already exists ($before); this check would change it"
    function Window($id) { [W]::TopLevel() | Where-Object { $o = 0; [void][W]::GetWindowThreadProcessId($_, [ref]$o); $o -eq $id -and [W]::Text($_) -eq 'Sarab' -and [W]::IsWindowVisible($_) } }
    try {
      Stop-Sarab
      if (Test-Path $sandbox) { Remove-Item $sandbox -Recurse -Force }
      New-Item -ItemType Directory -Force "$sandbox/roaming", "$sandbox/local" | Out-Null
      $env:APPDATA = "$sandbox/roaming"; $env:LOCALAPPDATA = "$sandbox/local"
      # 1. Login launch: tray only, Start with Windows untouched.
      $p = Start-Process $exe -ArgumentList '--autostart' -PassThru
      Wait-For { (Test-Path $statusFile) -and (Status).pid -eq $p.Id } 20 'login launch did not start'
      Start-Sleep 3
      Assert (-not (Window $p.Id)) 'a login launch opened the window'
      Assert (-not (Get-ItemProperty $run -ErrorAction SilentlyContinue).Sarab) 'a login launch turned on Start with Windows'
      # 2. Clicking Sarab while it runs in the background opens the window.
      Start-Process $exe | Out-Null
      Wait-For { Window $p.Id } 15 'launching Sarab while it runs did not open the window'
      Stop-Sarab
      # 3. First plain launch: window opens and Start with Windows turns on, pointing at this exe.
      $p = Start-Process $exe -PassThru
      Wait-For { Window $p.Id } 20 'a plain launch did not open the window'
      $val = (Get-ItemProperty $run -ErrorAction SilentlyContinue).Sarab
      Assert ($val -and $val -like "*sarab.exe*--autostart*") "Start with Windows not turned on by default: '$val'"
      Write-Host "start with Windows: $val"
      Stop-Sarab
      # 4. The user turns it off; later launches must not turn it back on.
      Remove-ItemProperty $run -Name Sarab
      $p = Start-Process $exe -PassThru
      Wait-For { Window $p.Id } 20 'second plain launch did not open the window'
      Start-Sleep 2
      Assert (-not (Get-ItemProperty $run -ErrorAction SilentlyContinue).Sarab) 'Start with Windows was forced back on after the user turned it off'
      Stop-Sarab
    } finally {
      Stop-Sarab
      $now = (Get-ItemProperty $run -ErrorAction SilentlyContinue).Sarab
      if ($now) { Remove-ItemProperty $run -Name Sarab }
      Remove-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run' -Name Sarab -ErrorAction SilentlyContinue
    }
    'launch gate passed'
  }

  default { Fail "unknown gate $Gate" }
}
