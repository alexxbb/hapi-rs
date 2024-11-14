import hou
import hapi
from pathlib import Path


session_info = hapi.SessionInfo()
cook_options = hapi.CookOptions()

session = hapi.createInProcessSession(session_info)
try:
    hapi.initialize(session, cook_options)
except hapi.AlreadyInitializedError:
    pass


file = Path(__file__).parent / "bone.idx"

buffer = open(str(file.absolute())).read()

count = hapi.getPresetCount(session, buffer, len(buffer))

handles = hapi.getPresetNames(session, buffer, len(buffer), count)

for h in handles:
    print(h)
    str_len = hapi.getStringBufLength(session, h)
    print(hapi.getString(session, h, str_len))
