using System.Diagnostics;
using System.Globalization;

namespace MyRigConfig.Tests;

internal sealed class HostNetworkFixture : IDisposable
{
    private readonly Process process;
    private HostNetworkFixture(Process process) => this.process = process;
    internal int ProcessId => process.Id;

    internal static async Task<HostNetworkFixture> StartAsync(int port)
    {
        var command = "$listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::IPv6Loopback, "
            + port.ToString(CultureInfo.InvariantCulture)
            + "); $listener.Server.DualMode = $false; try { $listener.Start(); "
            + "[Console]::WriteLine('READY'); [Console]::ReadLine() | Out-Null } finally { $listener.Stop() }";
        var start = new ProcessStartInfo("powershell.exe")
        {
            UseShellExecute = false, RedirectStandardInput = true,
            RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true,
        };
        foreach (var argument in new[] { "-NoProfile", "-NonInteractive", "-Command", command })
            start.ArgumentList.Add(argument);
        var fixture = new HostNetworkFixture(Process.Start(start)
            ?? throw new InvalidOperationException("Could not start the owned listener fixture."));
        try
        {
            Assert.AreEqual("READY", await fixture.process.StandardOutput.ReadLineAsync()
                .WaitAsync(TimeSpan.FromSeconds(10)));
            return fixture;
        }
        catch { fixture.Dispose(); throw; }
    }

    public void Dispose()
    {
        if (!process.HasExited)
        {
            process.StandardInput.WriteLine();
            process.StandardInput.Flush();
            if (!process.WaitForExit(5000)) process.Kill();
        }
        process.Dispose();
    }
}
