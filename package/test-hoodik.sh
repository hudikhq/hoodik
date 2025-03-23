#!/usr/bin/env bash

set -eo pipefail
set -x

case $1 in
  post-install)
    echo -e "\nHOODIK VERSION:"
    hoodik --version
    echo -e "\nHOODIK CONF:"
    cat /etc/hoodik.conf
    echo -e "\nHOODIK SERVICE STATUS BEFORE ENABLE:"
    systemctl status hoodik || true
    echo -e "\nENABLE HOODIK SERVICE:"
    systemctl enable hoodik
    echo -e "\nHOODIK SERVICE STATUS AFTER ENABLE:"
    systemctl status hoodik || true
    echo -e "\nSTART HOODIK SERVICE:"
    systemctl start hoodik
    
    echo -e "\nHOODIK SERVICE STATUS AFTER START:"
    sleep 1s
    systemctl status hoodik
    ;;

  post-upgrade)
    echo -e "\nHOODIK VERSION:"
    hoodik --version

    echo -e "\nHOODIK CONF:"
    cat /etc/hoodik.conf

    echo -e "\nHOODIK SERVICE STATUS:"
    systemctl status hoodik || true
    ;;
esac