#! /usr/bin/bash

APP=target/release/decode-metar
IN_OUT_PATH=tests/data/metar

plain_style_groups=("daytime")
cycles_style_groups=("cloud" "header" "present_weather" "pressure" "recent_weather" "rvr" "sea" "temperature" "visibility" "wind" "wind_shear")

for group in ${plain_style_groups[@]}; do
    ${APP} -f plain -p ${IN_OUT_PATH}/it_${group}_input.txt ${IN_OUT_PATH}/it_${group}_output.json
done

for group in ${cycles_style_groups[@]}; do
    ${APP} -p ${IN_OUT_PATH}/it_${group}_input.txt ${IN_OUT_PATH}/it_${group}_output.json
done
