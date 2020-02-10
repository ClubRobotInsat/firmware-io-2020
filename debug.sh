#!/bin/bash

Green='\033[0;42m'
Red='\033[0;41m'
End='\033[0;0m'

EXECUTABLE_FILE="target/thumbv7m-none-eabi/release/stm32-black-pill-rust"
check_gdb() {
    GDB=$(which "$1")
    if [ $? -eq "0" ] ; then
        echo -e "${Green}Launch a GDB session with '${GDB}'${End}"
        "$GDB" "$EXECUTABLE_FILE"
        exit $?
    else
        return 1
    fi
}

check_gdb gdb-multiarch 
check_gdb arm-none-eabi-gdb

echo -e "${Red}Impossible to launch a GDB session.${End}"

# A lancer à la main...
#st-util > st-link.log 2>&1 &
#arm-none-eabi-gdb target/thumbv7m-none-eabi/debug/stm32-black-pill-rust
#rust-gdb target/thumbv7em-none-eabihf/debug/nucleo_rust
