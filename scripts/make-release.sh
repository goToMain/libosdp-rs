#!/usr/bin/env bash
#
#  Copyright (c) 2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
#
#  SPDX-License-Identifier: Apache-2.0
#

usage() {
	cat >&2<<----
	LibOSDP release helper

	OPTIONS:
	  -c, --component	Compoenent to release (can be one of libosdp, libosdp-sys, osdpctl)
	  --patch		Release version bump type: patch (default)
	  --major		Release version bump type: major
	  --minor		Release version bump type: minor
	  -h, --help		Print this help
	---
}

function cargo_set_version() {
	dir=$1
	ver=$2
	perl -pi -se '
	if (/^version = "\d+\.\d+\.\d+"$/) {
		$_="version = \"$ver\"\n"
	}' -- -ver=$ver $dir/Cargo.toml
}

function cargo_inc_version() {
	dir=$1
	inc=$2
	perl -pi -se '
	if (/^version = "(\d+)\.(\d+)\.(\d+)"$/) {
		$maj=$1; $min=$2; $pat=$3;
		if ($major) { $maj+=1; $min=0; $pat=0; }
		if ($minor) { $min+=1; $pat=0; }
		$pat+=1 if $patch;
		$_="version = \"$maj.$min.$pat\"\n"
	}' -- -$inc $dir/Cargo.toml
}

function commit_release() {
	crate=$1
	version=$(perl -ne 'print $1 if (/^version = "(.+)"$/)' $crate/Cargo.toml)
	git add $crate/Cargo.toml &&
	git commit -s -m "$crate: Release v$version" &&
	git tag "$crate-v$version" -s -a -m "Release $version"
}

function do_cargo_release() {
	crate=$1
	inc=$2
	cargo_inc_version $crate $inc
	commit_release $crate
}

function do_libosdp_sys_bump() {
	latest_release=$(curl -s https://api.github.com/repos/gotoMain/libosdp/releases/latest | grep 'tag_name' | perl -pe 's|\s+"tag_name": "(.+)",|$1|')
	version=$(perl -ne 'print $1 if (/^version = "(.+)"$/)' libosdp-sys/Cargo.toml)
	if [[ "${latest_release}" == "v${version}" ]]; then
		echo "Nothing to be done"
		return
	fi
	pushd libosdp-sys/vendor
	git fetch origin
	git checkout ${latest_release}
	git submodule update --recursive
	popd
	git add libosdp-sys/vendor
	cargo_set_version libosdp-sys ${latest_release#"v"}
	commit_release libosdp-sys
}

function do_release() {
	case $1 in
	libosdp-sys) do_libosdp_sys_bump ;;
	libosdp) do_cargo_release "libosdp" $2 ;;
	osdpctl) do_cargo_release "osdpctl" $2 ;;
	esac
}

INC="patch"
COMPONENT="libosdp"
while [ $# -gt 0 ]; do
	case $1 in
	-c|--create)		CRATE=$2; shift;;
	--patch)		INC="patch";;
	--major)		INC="major";;
	--minor)		INC="minor";;
	-h|--help)             usage; exit 0;;
	*) echo -e "Unknown option $1\n"; usage; exit 1;;
	esac
	shift
done

do_release $CRATE $INC

