const std = @import("std");
const stdin = std.fs.File.stdin();
const stdout = std.fs.File.stdout();

pub fn main() !void {
    var stdin_buff: [1024]u8 = undefined;
    var stdout_buff: [1024]u8 = undefined;

    var r = stdin.readerStreaming(&stdin_buff);
    var w = stdout.writerStreaming(&stdout_buff);

    var r_need = try nextInt(i16, &r.interface);
    var g_need = try nextInt(i16, &r.interface);
    var b_need = try lastInt(i16, &r.interface);

    const r_have = try nextInt(i16, &r.interface);
    const g_have = try nextInt(i16, &r.interface);
    const b_have = try lastInt(i16, &r.interface);

    var rg_avail = try nextInt(i16, &r.interface);
    var gb_avail = try lastInt(i16, &r.interface);

    r_need = @max(r_need - r_have, 0);
    g_need = @max(g_need - g_have, 0);
    b_need = @max(b_need - b_have, 0);

    rg_avail -= r_need;
    gb_avail -= b_need;

    if (rg_avail < 0 or gb_avail < 0 or rg_avail + gb_avail < g_need) {
        try print("-1\n", .{}, &w.interface);
        return;
    }

    try print("{d}\n", .{r_need + g_need + b_need}, &w.interface);
}

fn lastInt(comptime T: type, r: *std.Io.Reader) !T {
    const token = try r.takeDelimiterExclusive('\n');
    _ = try r.take(1);
    return try std.fmt.parseInt(T, token, 10);
}

fn nextInt(comptime T: type, r: *std.Io.Reader) !T {
    const token = try r.takeDelimiterExclusive(' ');
    _ = try r.take(1);
    return try std.fmt.parseInt(T, token, 10);
}

fn print(comptime fmt: []const u8, args: anytype, w: *std.Io.Writer) !void {
    try w.print(fmt, args);
    try w.flush();
}
