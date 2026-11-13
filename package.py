#!/usr/bin/env python3
"""Build the reproducible ClickThrough native mod archive."""

import json
from pathlib import Path
import tomllib
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

ROOT = Path(__file__).resolve().parent
BUNDLE = ROOT
MIXINS_REVISION = "e59da9094f0c2fdda6c627fe9eff22dec6a906f0"


def main():
    manifest = tomllib.loads((BUNDLE / "horizon-mixin.toml").read_text())
    mod = manifest["mod"]
    metadata = {
        "schema": 1,
        "id": mod["id"],
        "version": mod["version"],
        "game": "26.3",
        "horizon": manifest["compatibility"]["horizon_revision"],
        "mixins": MIXINS_REVISION,
    }
    entries = {
        f"mixins/clickthrough/{file.name}": file.read_bytes()
        for file in BUNDLE.iterdir()
        if file.suffix in {".toml", ".rs"}
    }
    entries["horizon-native.json"] = (json.dumps(metadata, indent=2) + "\n").encode()
    entries["mixins/clickthrough/LICENSE"] = (ROOT / "LICENSE").read_bytes()

    output = ROOT / "dist" / f"{mod['id']}-{mod['version']}.hnmod"
    output.parent.mkdir(exist_ok=True)
    with ZipFile(output, "w") as archive:
        for name, content in sorted(entries.items()):
            info = ZipInfo(name)
            info.compress_type = ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, content)
    print(output)


if __name__ == "__main__":
    main()
