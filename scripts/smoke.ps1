$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    npx tauri build --debug --no-bundle
    if ($LASTEXITCODE -ne 0) { throw "tauri build failed with exit code $LASTEXITCODE" }

    Add-Type @'
using System; using System.Text; using System.Runtime.InteropServices;
public static class SmokeWin {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowThreadProcessId(IntPtr h, out int pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out RECT r, int size);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern long GetWindowLongPtr(IntPtr h, int i);
  [DllImport("user32.dll")] public static extern bool GetLayeredWindowAttributes(IntPtr h, out uint key, out byte alpha, out uint flags);
  public struct RECT { public int L, T, R, B; }
  static bool Shown(IntPtr h) {
    uint key, flags; byte alpha;
    bool clickThrough = (GetWindowLongPtr(h, -20) & 0x20) != 0;
    return IsWindowVisible(h) && !clickThrough && GetLayeredWindowAttributes(h, out key, out alpha, out flags) && alpha == 255;
  }
  public static string Find(int pid) {
    string found = null;
    EnumWindows((h, l) => {
      int p; GetWindowThreadProcessId(h, out p);
      var t = new StringBuilder(64); GetWindowText(h, t, 64);
      if (p == pid && t.ToString() == "Openray" && Shown(h)) {
        RECT r; DwmGetWindowAttribute(h, 9, out r, 16);
        found = String.Format("{0}x{1}@{2}", r.R - r.L, r.B - r.T, GetDpiForWindow(h));
        return false;
      }
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
'@
    [SmokeWin]::SetProcessDPIAware() | Out-Null

    $exe = Join-Path $root 'src-tauri\target\debug\openray.exe'
    $proc = Start-Process $exe -ArgumentList '--show' -PassThru
    try {
        $found = $null
        for ($i = 0; $i -lt 50 -and -not $found; $i++) {
            Start-Sleep -Milliseconds 200
            $found = [SmokeWin]::Find($proc.Id)
        }
        if (-not $found) { throw 'launcher window did not become visible within 10 s' }
        $size, $dpi = $found -split '@'
        $scale = [int]$dpi / 96
        $expected = '{0}x{1}' -f [math]::Round(750 * $scale), [math]::Round(475 * $scale)
        if ($size -ne $expected) { throw "launcher is $size at $dpi DPI, expected $expected" }
        Write-Host "SMOKE OK: launcher visible, $size at $dpi DPI"
    }
    finally {
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    }
}
finally {
    Pop-Location
}
