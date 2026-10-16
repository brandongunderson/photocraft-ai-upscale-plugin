"""Run real packaged workers; optional large cases exercise the reported 4x photo size."""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import struct
import subprocess
import time
import zipfile
import zlib

from PIL import Image, ImageChops


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def pe_imports(data):
    """Read the normal PE import table without relying on developer tools in PATH."""
    assert data[:2] == b"MZ"
    pe = struct.unpack_from("<I", data, 60)[0]
    assert data[pe:pe + 4] == b"PE\0\0"
    machine, sections = struct.unpack_from("<HH", data, pe + 4)
    assert machine == 0x8664
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    optional = pe + 24
    assert struct.unpack_from("<H", data, optional)[0] == 0x20b
    import_rva, _ = struct.unpack_from("<II", data, optional + 112 + 8)
    section_table = optional + optional_size

    def offset(rva):
        for index in range(sections):
            virtual_size, virtual, raw_size, raw = struct.unpack_from(
                "<IIII", data, section_table + index * 40 + 8)
            if virtual <= rva < virtual + max(virtual_size, raw_size):
                result = raw + rva - virtual
                assert result < len(data), "Invalid PE import offset"
                return result
        raise AssertionError("Invalid PE import address")

    imports = []
    descriptor = offset(import_rva)
    for _ in range(256):
        entry = struct.unpack_from("<IIIII", data, descriptor)
        if not any(entry):
            return imports
        name = offset(entry[3])
        end = data.index(b"\0", name, min(name + 256, len(data)))
        imports.append(data[name:end].decode("ascii"))
        descriptor += 20
    raise AssertionError("Unbounded PE import table")


def inspect_package(package):
    with zipfile.ZipFile(package) as archive:
        assert archive.testzip() is None
        names = archive.namelist()
        assert len(names) == len({name.casefold() for name in names}), "Duplicate/colliding paths"
        for name in names:
            path = PurePosixPath(name)
            assert "\\" not in name and not path.is_absolute() and ".." not in path.parts
            assert not name.endswith("/") and ":" not in path.parts[0]
        assert json.loads(archive.read("package.json")) == {
            "packageVersion": 2, "module": "plugin.pcext",
            "native": {"service": "plugin.processImage", "platform": "windows-x86_64",
                       "entry": "photocraft-upscale-worker.exe"},
        }
        assert archive.read("plugin.pcext").startswith(b"\0asm\x01\0\0\0")
        worker = archive.read("photocraft-upscale-worker.exe")
        assert worker.startswith(b"MZ")
        offset = struct.unpack_from("<I", worker, 60)[0]
        assert worker[offset:offset + 4] == b"PE\0\0"
        assert struct.unpack_from("<H", worker, offset + 4)[0] == 0x8664
        assets = json.loads(archive.read("runtime/downloads.json"))
        verified = 0
        for asset in assets["files"]:
            if asset.get("default"):
                digest = hashlib.sha256(archive.read("runtime/" + asset["path"])).hexdigest()
                assert digest == asset["sha256"], "Runtime checksum mismatch: " + asset["path"]
                verified += 1
        os_libraries = {"kernel32.dll", "msvcrt.dll", "ntdll.dll", "bcryptprimitives.dll",
                        "api-ms-win-core-synch-l1-2-0.dll", "ole32.dll", "oleaut32.dll"}
        bundled = {PurePosixPath(name).name.casefold(): name for name in names}
        imports = {}
        for name in ["photocraft-upscale-worker.exe", "runtime/bin/upscayl-bin.exe",
                     "runtime/bin/vcomp140.dll"]:
            dependencies = pe_imports(archive.read(name))
            for dependency in dependencies:
                lower = dependency.casefold()
                assert lower in os_libraries or lower in bundled or (
                    name == "runtime/bin/upscayl-bin.exe" and lower == "vulkan-1.dll"
                ), "Unbundled non-system native dependency: " + dependency
            imports[name] = dependencies
        return {"archive_files": len(names), "crc": "passed",
                "runtime_hashes_verified": verified, "package_contract": "passed",
                "portable_paths": "passed", "x64_worker_and_wasm_headers": "passed",
                "native_dependency_imports": imports,
                "external_native_requirements": "Windows OS libraries and the GPU driver's Vulkan loader only"}


class MemoryCounters(ctypes.Structure):
    _fields_ = [("cb", ctypes.c_ulong), ("PageFaultCount", ctypes.c_ulong)] + [
        (name, ctypes.c_size_t)
        for name in [
            "PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage",
            "QuotaPagedPoolUsage", "QuotaPeakNonPagedPoolUsage", "QuotaNonPagedPoolUsage",
            "PagefileUsage", "PeakPagefileUsage", "PrivateUsage",
        ]
    ]


def worker_memory(process):
    if os.name != "nt":
        return 0, 0
    memory = MemoryCounters()
    memory.cb = ctypes.sizeof(memory)
    okay = ctypes.windll.psapi.GetProcessMemoryInfo(
        ctypes.c_void_p(int(process._handle)), ctypes.byref(memory), memory.cb
    )
    return (memory.PeakWorkingSetSize, memory.PeakPagefileUsage) if okay else (0, 0)


def run_case(root, work, name, source, engine, scale, model="upscayl-lite-4x", alpha=None):
    output = work / (name + ".png")
    output.unlink(missing_ok=True)
    request = work / (name + ".json")
    with Image.open(source) as image:
        original = image.size
    request.write_text(json.dumps({
        "apiVersion": 1, "input": str(source), "output": str(output),
        "options": {"engine": engine, "model": model, "scale": scale,
                    "detail": 100, "tile_size": 128, "tta": False},
        "outputSize": {"width": original[0] * scale, "height": original[1] * scale},
        "colorSpace": "srgb",
    }), encoding="utf-8")
    before = sha256(source)
    started = time.perf_counter()
    peak_working, peak_commit = 0, 0
    executable = str(root / "photocraft-upscale-worker.exe")
    if os.name == "nt" and len(executable) >= 260:
        executable = "\\\\?\\" + executable
    with (work / (name + ".log")).open("wb") as log:
        environment = os.environ.copy()
        if os.name == "nt":
            windows = environment.get("SystemRoot", r"C:\Windows")
            environment["PATH"] = windows + os.pathsep + str(Path(windows) / "System32")
        process = subprocess.Popen(
            [executable, "--job", str(request)],
            executable=executable,
            stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=log,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            env=environment,
        )
        while process.poll() is None:
            working, commit = worker_memory(process)
            peak_working, peak_commit = max(working, peak_working), max(commit, peak_commit)
            if time.perf_counter() - started > 600:
                process.kill()
                process.wait()
                raise RuntimeError("Worker timed out: " + name)
            time.sleep(0.05)
        working, commit = worker_memory(process)
        peak_working, peak_commit = max(working, peak_working), max(commit, peak_commit)
        process.stdin.close()
    elapsed = time.perf_counter() - started
    if process.returncode != 0:
        raise RuntimeError("Worker failed: " + name + ": " + (work / (name + ".log")).read_text())
    assert sha256(source) == before, "Source changed"
    changed_rgb = None
    with Image.open(output) as result:
        assert result.size == (original[0] * scale, original[1] * scale), result.size
        assert result.mode == "RGBA" and result.info.get("icc_profile"), "Missing alpha or ICC"
        if alpha is not None:
            assert result.getchannel("A").getextrema() == (alpha, alpha), "Alpha changed"
        else:
            with Image.open(source) as image:
                original_alpha = image.getchannel("A")
                if scale == 1:
                    assert result.getchannel("A").tobytes() == original_alpha.tobytes()
                    if engine == "ai":
                        original_pixels, processed_pixels = image.tobytes(), result.tobytes()
                        changed_rgb = sum(original_pixels[i:i + 3] != processed_pixels[i:i + 3]
                                          for i in range(0, len(original_pixels), 4))
                        if source.name == "small-source.png":
                            assert changed_rgb > 0, "The AI fixture was copied without reconstruction"
                else:
                    expected = original_alpha.resize(result.size, Image.Resampling.LANCZOS)
                    assert ImageChops.difference(result.getchannel("A"), expected).getextrema()[1] <= 1
        result_size = result.size
    entry = {"name": name, "engine": engine, "model": model if engine == "ai" else None,
             "input_size": original, "output_size": result_size, "scale": scale,
             "seconds": round(elapsed, 3), "peak_worker_working_bytes": peak_working,
             "peak_worker_commit_bytes": peak_commit, "source_unchanged": True,
             "alpha": "verified", "icc": "present", "status": "passed"}
    if os.name == "nt":
        entry["minimal_path"] = "Windows and System32 only"
    if changed_rgb is not None:
        entry["changed_rgb_pixels"] = changed_rgb
    print(json.dumps(entry), flush=True)
    return entry


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--large", action="store_true")
    parser.add_argument("--long-path", action="store_true")
    args = parser.parse_args()
    package_checks = inspect_package(args.package)
    Image.MAX_IMAGE_PIXELS = None
    project = Path(__file__).resolve().parents[1]
    version = json.loads((project / "extension/manifest.json").read_text(encoding="utf-8"))["version"]
    work = project / "target" / ("v" + version + "-package-smoke")
    root = work / "installed"
    root.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(args.package) as archive:
        assert archive.testzip() is None
        # This tool consumes only the locally built release, whose members are still checked.
        for member in archive.infolist():
            destination = (root / member.filename).resolve()
            assert destination.is_relative_to(root.resolve()), "Archive path traversal"
        archive.extractall(root)
    source = work / "small-source.png"
    image = Image.new("RGBA", (16, 12))
    image.putdata([(x * 15, y * 21, (x * 9 + y * 5) % 256, 40 + (x + y) * 7)
                   for y in range(12) for x in range(16)])
    image.save(source)
    checks = []
    models = sorted({
        Path(asset["path"]).stem
        for asset in json.loads((root / "runtime/downloads.json").read_text(encoding="utf-8"))["files"]
        if asset.get("default") and asset["path"].startswith("models/") and asset["path"].endswith(".param")
    })
    assert len(models) == 7, "The seven-model release must include every model"
    for model in models:
        for scale in [1, 2]:
            checks.append(run_case(root, work, model + "-" + str(scale), source, "ai", scale, model))
    checks.append(run_case(root, work, "software-1x", source, "software", 1))
    if args.large:
        for name, size, scale in [("24mp-software-1x", (6000, 4000), 1),
                                  ("reported-photo-software-4x", (4240, 2832), 4)]:
            source = work / (name + "-source.png")
            image = Image.new("RGBA", size, (120, 80, 40, 170))
            image.paste((200, 150, 80, 170), (size[0] // 2, 0, size[0], size[1]))
            image.save(source)
            image.close()
            checks.append(run_case(root, work, name, source, "software", scale, alpha=170))
    if args.long_path:
        assert os.name == "nt", "This regression exercises the Windows runtime path limit"
        long_root = work / ("managed-install-" + "a" * 60) / ("generation-" + "b" * 64)
        long_root.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(args.package) as archive:
            archive.extractall(long_root)
        source = work / "long-managed-white-rgba8.png"
        with Image.open(work / "upscayl-standard-4x-1.png") as previous:
            profile = previous.info["icc_profile"]
        def chunk(kind, data):
            return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xffffffff)
        source.write_bytes(b"\x89PNG\r\n\x1a\n" +
                           chunk(b"IHDR", struct.pack(">IIBBBBB", 32, 24, 8, 6, 0, 0, 0)) +
                           chunk(b"iCCP", b"sRGB\0\0" + zlib.compress(profile)) +
                           chunk(b"IDAT", zlib.compress((b"\0" + b"\xff" * (32 * 4)) * 24)) +
                           chunk(b"IEND", b""))
        model_path = long_root / "runtime/models/upscayl-standard-4x.param"
        assert len(str(model_path)) > 260
        entry = run_case(long_root, work, "long-managed-standard-white8-1x", source,
                         "ai", 1, "upscayl-standard-4x", alpha=255)
        entry["installed_worker_path_characters"] = len(str(long_root / "photocraft-upscale-worker.exe"))
        entry["installed_model_path_characters"] = len(str(model_path))
        entry["input_depth"] = 8
        checks.append(entry)
    report = {"plugin_version": version, "package_bytes": args.package.stat().st_size,
              "sha256": sha256(args.package), **package_checks,
              "memory_measurement": "Peak worker process working set and commit; excludes GPU and child process memory.",
              "checks": checks}
    (project / "verification" / "package-smoke.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
