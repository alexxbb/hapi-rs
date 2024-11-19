import sys
import hou
import hapi
from pathlib import Path


session_info = hapi.SessionInfo()
cook_options = hapi.CookOptions()

try:
    preset_file = sys.argv[1]
except IndexError:
    print("Must pass .idx file")
    sys.exit(1)

session = hapi.createInProcessSession(session_info)
try:
    hapi.initialize(session, cook_options)
except hapi.AlreadyInitializedError:
    pass


preset_file = Path(preset_file).absolute().as_posix()

buffer = open(preset_file).read()

count = hapi.getPresetCount(session, buffer, len(buffer))
print(f"Num presets: {count}")

handles = hapi.getPresetNames(session, buffer, len(buffer), count)

for h in handles:
    str_len = hapi.getStringBufLength(session, h)
    print(" -" + hapi.getString(session, h, str_len))
