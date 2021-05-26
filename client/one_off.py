import time
import sys

# Script executed from run_script.hda


def main():
    time.sleep(10)
    print("Hello %s", __file__)
    sys.stdout.flush()
