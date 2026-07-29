param(
  [Parameter(Mandatory = $true)]
  [ValidateSet("Install", "Remove")]
  [string]$Action,
  [string]$TargetPath,
  [string]$Arguments,
  [string]$AppId,
  [Parameter(Mandatory = $true)]
  [string]$ShortcutName
)

$ErrorActionPreference = "Stop"
$shortcutPath = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$ShortcutName.lnk"

if ($Action -eq "Remove") {
  Remove-Item -Force -ErrorAction SilentlyContinue $shortcutPath
  exit 0
}

if (-not $TargetPath -or -not $AppId) {
  throw "TargetPath and AppId are required when installing the shortcut"
}

New-Item -ItemType Directory -Force -Path (Split-Path $shortcutPath) | Out-Null

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;

namespace NotifyStatusFixture
{
    [ComImport]
    [Guid("00021401-0000-0000-C000-000000000046")]
    internal class ShellLink { }

    [ComImport]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    [Guid("000214F9-0000-0000-C000-000000000046")]
    internal interface IShellLinkW
    {
        void GetPath([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder file, int maxPath, IntPtr findData, uint flags);
        void GetIDList(out IntPtr itemIdList);
        void SetIDList(IntPtr itemIdList);
        void GetDescription([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder name, int maxName);
        void SetDescription([MarshalAs(UnmanagedType.LPWStr)] string name);
        void GetWorkingDirectory([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder directory, int maxPath);
        void SetWorkingDirectory([MarshalAs(UnmanagedType.LPWStr)] string directory);
        void GetArguments([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder arguments, int maxPath);
        void SetArguments([MarshalAs(UnmanagedType.LPWStr)] string arguments);
        void GetHotkey(out ushort hotkey);
        void SetHotkey(ushort hotkey);
        void GetShowCmd(out int showCommand);
        void SetShowCmd(int showCommand);
        void GetIconLocation([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder iconPath, int maxPath, out int iconIndex);
        void SetIconLocation([MarshalAs(UnmanagedType.LPWStr)] string iconPath, int iconIndex);
        void SetRelativePath([MarshalAs(UnmanagedType.LPWStr)] string path, uint reserved);
        void Resolve(IntPtr window, uint flags);
        void SetPath([MarshalAs(UnmanagedType.LPWStr)] string file);
    }

    [ComImport]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    [Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99")]
    internal interface IPropertyStore
    {
        uint GetCount();
        PropertyKey GetAt(uint index);
        void GetValue(ref PropertyKey key, out PropVariant value);
        void SetValue(ref PropertyKey key, ref PropVariant value);
        void Commit();
    }

    [ComImport]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    [Guid("0000010B-0000-0000-C000-000000000046")]
    internal interface IPersistFile
    {
        void GetClassID(out Guid classId);
        [PreserveSig] int IsDirty();
        void Load([MarshalAs(UnmanagedType.LPWStr)] string fileName, uint mode);
        void Save([MarshalAs(UnmanagedType.LPWStr)] string fileName, [MarshalAs(UnmanagedType.Bool)] bool remember);
        void SaveCompleted([MarshalAs(UnmanagedType.LPWStr)] string fileName);
        void GetCurFile([MarshalAs(UnmanagedType.LPWStr)] out string fileName);
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct PropertyKey
    {
        internal Guid FormatId;
        internal uint PropertyId;
    }

    [StructLayout(LayoutKind.Explicit, Size = 24)]
    internal struct PropVariant
    {
        [FieldOffset(0)] internal ushort VariantType;
        [FieldOffset(8)] internal IntPtr PointerValue;
    }

    public static class ShortcutRegistration
    {
        public static void Install(string shortcutPath, string targetPath, string arguments, string appId)
        {
            object shellLinkObject = new ShellLink();
            IntPtr appIdPointer = IntPtr.Zero;

            try
            {
                IShellLinkW shellLink = (IShellLinkW)shellLinkObject;
                shellLink.SetPath(targetPath);
                shellLink.SetArguments(arguments ?? "");
                shellLink.SetWorkingDirectory(System.IO.Path.GetDirectoryName(targetPath));

                PropertyKey appUserModelId = new PropertyKey
                {
                    FormatId = new Guid("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"),
                    PropertyId = 5
                };
                appIdPointer = Marshal.StringToCoTaskMemUni(appId);
                PropVariant appIdValue = new PropVariant
                {
                    VariantType = 31,
                    PointerValue = appIdPointer
                };

                IPropertyStore propertyStore = (IPropertyStore)shellLinkObject;
                propertyStore.SetValue(ref appUserModelId, ref appIdValue);
                propertyStore.Commit();

                ((IPersistFile)shellLinkObject).Save(shortcutPath, true);
            }
            finally
            {
                if (appIdPointer != IntPtr.Zero)
                {
                    Marshal.FreeCoTaskMem(appIdPointer);
                }
                if (Marshal.IsComObject(shellLinkObject))
                {
                    Marshal.FinalReleaseComObject(shellLinkObject);
                }
            }
        }
    }
}
'@

[NotifyStatusFixture.ShortcutRegistration]::Install($shortcutPath, $TargetPath, $Arguments, $AppId)
Write-Output $shortcutPath
