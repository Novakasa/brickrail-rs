from pathlib import Path
from pybricksdev.compile import compile_multi_file
import asyncio

async def main():
    program_root = Path(__file__).parent / "programs"

    for program in program_root.glob("*.py"):
        print(f"Compiling {program}...")
        mpy = await compile_multi_file(str(program), (6,1))
        mpy_path = (program_root / "mpy" / program.name).with_suffix(".mpy")
        print(f"Writing {mpy_path}...")
        with open(mpy_path, "wb") as f:
            f.write(mpy)

asyncio.run(main())