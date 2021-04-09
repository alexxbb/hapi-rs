import hashlib
import random
import json
import time

import requests
import pydantic

URL = 'http://localhost:3030'


class Task(pydantic.BaseModel):
    show: str


# def get_sessions():
#     res = requests.get(URL + "/sessions")
#     if res.status_code != 200:
#         print(res.text)
#         return
#     print(res.json())
#     # r = pydantic.parse_obj_as(list[Session], res.json())
#     # for s in r:
#     #     print(s)


shows = {'wam', 'cbf', 'jsb', 'viv', 'ht4', 'vhe'}


def send_task(show):
    res = requests.post(URL + "/jobs", data=Task(show=show).json())
    if res.status_code != 200:
        print('ERROR: ', res.text)
    else:
        print(f'{show} OK: ', res.text)


for s in shows:
    send_task(s)
# print("Job send")

# get_sessions()
