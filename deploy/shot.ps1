param(
    [int]$ProcessId,
    [string]$Out = "shot.png"
)

# Grabs a window without touching the keyboard or stealing focus.
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
[void][Win]::SetProcessDPIAware()

$handle = (Get-Process -Id $ProcessId).MainWindowHandle
if ($handle -eq [IntPtr]::Zero) { throw "no window for pid $ProcessId" }

$rect = New-Object Win+RECT
[void][Win]::GetWindowRect($handle, [ref]$rect)
$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top

$bitmap = New-Object System.Drawing.Bitmap $width, $height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$dc = $graphics.GetHdc()
# 2 = PW_RENDERFULLCONTENT, needed for GPU-composited windows.
[void][Win]::PrintWindow($handle, $dc, 2)
$graphics.ReleaseHdc($dc)
$graphics.Dispose()
$bitmap.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$bitmap.Dispose()
"$Out ${width}x${height}"
