using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.Versioning;
using Windows.Win32;
using Windows.Win32.NetworkManagement.IpHelper;

using static MyRigConfig.Core.Host.HostNetworkConstants;

namespace MyRigConfig.Core.Host;

/// <summary>Which process listens on a TCP port, from the Windows TCP tables (IPv4 and IPv6).</summary>
[SupportedOSPlatform("windows10.0")]
internal static partial class HostNetworkNative
{
    internal static int? FindListener(int port, int? excludedProcessId) =>
        FindListener(port, AddressFamilyInet, Ipv4RowSize, Ipv4PortOffset, Ipv4PidOffset, excludedProcessId)
        ?? FindListener(port, AddressFamilyInet6, Ipv6RowSize, Ipv6PortOffset, Ipv6PidOffset, excludedProcessId);

    /// <summary>A process's executable name, or null if Windows won't tell us.</summary>
    internal static string? ProcessName(int processId)
    {
        try
        {
            using var process = Process.GetProcessById(processId);
            return process.ProcessName;
        }
        catch (Exception exception) when (exception is ArgumentException or InvalidOperationException
            or System.ComponentModel.Win32Exception)
        {
            return null;
        }
    }

    // ! If Windows can't give us the table, that's an error, never "free", so the caller reports that the port couldn't
    // ! be checked.
    private static int? FindListener(int port, int family, int rowSize, int portOffset, int pidOffset,
        int? excludedProcessId)
    {
        uint size = 0;
        PInvoke.GetExtendedTcpTable(Span<byte>.Empty, ref size, false, (uint)family,
            (TCP_TABLE_CLASS)TcpTableOwnerPidListener, 0);
        for (var attempt = 0; attempt < 3 && size > 0; attempt++)
        {
            if (size > int.MaxValue) throw new Win32Exception();
            var buffer = new byte[(int)size];
            var result = PInvoke.GetExtendedTcpTable(buffer, ref size, false, (uint)family,
                (TCP_TABLE_CLASS)TcpTableOwnerPidListener, 0);
            if (result == ErrorInsufficientBuffer) continue;
            if (result != 0) throw new Win32Exception((int)result);
            if (buffer.Length < sizeof(int)) throw new Win32Exception();
            var count = BitConverter.ToInt32(buffer);
            if (count < 0 || count > (buffer.Length - sizeof(int)) / rowSize) throw new Win32Exception();
            for (var index = 0; index < count; index++)
            {
                var offset = sizeof(int) + index * rowSize;
                var raw = BitConverter.ToInt32(buffer, offset + portOffset);
                var rowPort = ((raw & 0xFF) << 8) | ((raw >> 8) & 0xFF);
                if (rowPort != port) continue;
                var owner = BitConverter.ToInt32(buffer, offset + pidOffset);
                if (owner != excludedProcessId) return owner;
            }
            return null;
        }
        throw new Win32Exception();
    }
}
