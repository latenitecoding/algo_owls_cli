const std = @import("std");
const stdin = std.fs.File.stdin();
const stdout = std.fs.File.stdout();

pub fn main() !void {
    var in: [1024]u8 = undefined;
    var out: [1024]u8 = undefined;
    var r = stdin.readerStreaming(&in);
    var w = stdout.writerStreaming(&out);
    var line = try nextLine(&r.interface);
    try print("{s}\n", &w.interface, .{line});
    line = try nextLine(&r.interface);
    try print("{s}\n", &w.interface, .{line});
    line = try nextLine(&r.interface);
    try print("{s}\n", &w.interface, .{line});
}

fn nextLine(r: *std.Io.Reader) ![]u8 {
    return try r.takeDelimiterExclusive('\n');
}

fn print(comptime fmt: []const u8, w: *std.Io.Writer, args: anytype) !void {
    try w.print(fmt, args);
    try w.flush();
}
