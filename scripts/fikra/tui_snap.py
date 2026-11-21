"""Run a TUI in a pty, answer terminal queries, and print the rendered screen.
Usage: tui_snap.py <seconds> <keys-after-start|-> <cmd...>"""
import os, pty, select, sys, time, re
import pyte
secs = float(sys.argv[1]); keys = sys.argv[2]; cmd = sys.argv[3:]
cols, rows = 110, 36
screen = pyte.Screen(cols, rows); stream = pyte.ByteStream(screen)
pid, fd = pty.fork()
if pid == 0:
    os.environ.update({"TERM": "xterm-256color", "COLUMNS": str(cols), "LINES": str(rows)})
    os.execvp(cmd[0], cmd)
import fcntl, termios, struct
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
end = time.time() + secs; sent = False; snaps = []
seen_frames = []; last_frame_at = 0.0
while time.time() < end:
    r, _, _ = select.select([fd], [], [], 0.2)
    if r:
        try: data = os.read(fd, 65536)
        except OSError: break
        if b"\x1b[6n" in data: os.write(fd, b"\x1b[1;1R")
        if b"\x1b[c" in data: os.write(fd, b"\x1b[?62;22c")
        if b"\x1b]10;?" in data: os.write(fd, b"\x1b]10;rgb:ffff/ffff/ffff\x1b\\")
        if b"\x1b]11;?" in data: os.write(fd, b"\x1b]11;rgb:0000/0000/0000\x1b\\")
        if b"\x1b[?u" in data: os.write(fd, b"\x1b[?0u")
        stream.feed(data)
    if time.time() - last_frame_at > 0.2:
        last_frame_at = time.time()
        frame = "\n".join(l.rstrip() for l in screen.display if l.strip())
        if frame and (not seen_frames or seen_frames[-1] != frame):
            seen_frames.append(frame)
    if not sent and keys != "-" and time.time() > end - secs/2:
        os.write(fd, keys.encode().decode("unicode_escape").encode()); sent = True
        snaps.append("\n".join(screen.display))
print("\n".join(l.rstrip() for l in (snaps[0] if snaps else "").split("\n") if l.strip()))
print("=" * 40)
print("\n".join(l.rstrip() for l in screen.display if l.strip()))
# Every distinct frame, so a model or logo shown only briefly is still caught.
print("=" * 16 + " all frames " + "=" * 16)
print("\n".join(seen_frames))
try: os.kill(pid, 9)
except Exception: pass
