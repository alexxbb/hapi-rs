_default:
    @just --list


start_hars:
    @echo "Starting HARS"
    $HFS/bin/HARS -n $TMP/hars.pipe