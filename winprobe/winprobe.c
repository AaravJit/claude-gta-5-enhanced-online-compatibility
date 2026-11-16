/*
 * winprobe - a self-owned, read-only Windows API probe.
 *
 * Purpose: show, with a program we wrote ourselves, what a Wine/Proton prefix
 * reports through ordinary documented Windows APIs, and which "kernel"
 * facilities exist there. It demonstrates the Layer 1 / Layer 3 distinction
 * in docs/RESEARCH.md without touching any game or anti-cheat component.
 *
 * It only queries. It creates no services, loads no drivers, writes no files
 * and opens no other process.
 *
 * Build (Arch: pacman -S mingw-w64-gcc):
 *   x86_64-w64-mingw32-gcc -O2 -Wall -Wextra -municode -o winprobe.exe winprobe.c -ladvapi32
 * Run in a THROWAWAY prefix, never the game prefix:
 *   WINEPREFIX=$HOME/.cache/gtaol-probe wine winprobe.exe
 */
#include <windows.h>
#include <winternl.h>
#include <stdio.h>

typedef LONG(WINAPI *RtlGetVersion_t)(OSVERSIONINFOEXW *);
typedef LONG(WINAPI *NtQuerySystemInformation_t)(ULONG, void *, ULONG, ULONG *);
typedef const char *(CDECL *wine_get_version_t)(void);

struct code_integrity { ULONG Length; ULONG CodeIntegrityOptions; };

static void probe_version(HMODULE ntdll)
{
    RtlGetVersion_t get = (RtlGetVersion_t)(void *)GetProcAddress(ntdll, "RtlGetVersion");
    wine_get_version_t wine = (wine_get_version_t)(void *)GetProcAddress(ntdll, "wine_get_version");
    OSVERSIONINFOEXW v = { .dwOSVersionInfoSize = sizeof(v) };

    if (get && get(&v) == 0)
        printf("os.version            %lu.%lu build %lu\n", v.dwMajorVersion, v.dwMinorVersion, v.dwBuildNumber);
    /* Wine does not hide itself: this export is public and any program may call it. */
    printf("ntdll.wine_get_version %s\n", wine ? wine() : "(absent: real Windows)");
}

static void probe_code_integrity(HMODULE ntdll)
{
    NtQuerySystemInformation_t q = (NtQuerySystemInformation_t)(void *)GetProcAddress(ntdll, "NtQuerySystemInformation");
    struct code_integrity ci = { .Length = sizeof(ci) };
    ULONG len = 0;
    LONG st = q ? q(103 /* SystemCodeIntegrityInformation */, &ci, sizeof(ci), &len) : -1;

    if (st == 0)
        printf("code_integrity         options=0x%08lx\n", ci.CodeIntegrityOptions);
    else
        printf("code_integrity         unavailable (NTSTATUS 0x%08lx)\n", (ULONG)st);
}

static void probe_drivers(void)
{
    SC_HANDLE scm = OpenSCManagerW(NULL, NULL, SC_MANAGER_ENUMERATE_SERVICE);
    BYTE *buf = NULL;
    DWORD need = 0, count = 0, resume = 0, i;

    if (!scm) {
        printf("scm                    cannot open (error %lu)\n", GetLastError());
        return;
    }
    EnumServicesStatusExW(scm, SC_ENUM_PROCESS_INFO, SERVICE_DRIVER, SERVICE_STATE_ALL, NULL, 0, &need, &count, &resume, NULL);
    buf = need ? HeapAlloc(GetProcessHeap(), 0, need) : NULL;
    if (buf && EnumServicesStatusExW(scm, SC_ENUM_PROCESS_INFO, SERVICE_DRIVER, SERVICE_STATE_ALL, buf, need, &need, &count, &resume, NULL)) {
        ENUM_SERVICE_STATUS_PROCESSW *s = (ENUM_SERVICE_STATUS_PROCESSW *)buf;
        printf("scm.driver_services    %lu registered\n", count);
        for (i = 0; i < count; i++)
            printf("  driver %-24ls state=%lu\n", s[i].lpServiceName, s[i].ServiceStatusProcess.dwCurrentState);
    } else {
        printf("scm.driver_services    enumeration failed (error %lu)\n", GetLastError());
    }
    if (buf) HeapFree(GetProcessHeap(), 0, buf);
    CloseServiceHandle(scm);
}

int wmain(void)
{
    HMODULE ntdll = GetModuleHandleW(L"ntdll.dll");

    printf("winprobe: read-only Windows API probe\n");
    probe_version(ntdll);
    probe_code_integrity(ntdll);
    probe_drivers();
    printf("note: under Wine, \"driver\" services run inside winedevice.exe, an ordinary\n"
           "      Linux user process. Nothing listed above executes in the Linux kernel.\n");
    return 0;
}
