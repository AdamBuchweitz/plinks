#!/usr/bin/env bash

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
template_path="${repo_root}/packaging/arch/PKGBUILD.in"
output_dir="${repo_root}/dist/arch"

if [[ ! -d "${repo_root}/.git" ]]; then
  echo "error: ${repo_root} is not a git repository" >&2
  exit 1
fi

if [[ ! -f "${template_path}" ]]; then
  echo "error: missing template ${template_path}" >&2
  exit 1
fi

package_name=$(
  sed -n 's/^name = "\(.*\)"/\1/p' "${repo_root}/Cargo.toml" | head -n1
)

version=$(
  sed -n 's/^version = "\(.*\)"/\1/p' "${repo_root}/Cargo.toml" | head -n1
)

if [[ -z "${package_name}" ]]; then
  echo "error: failed to read package name from Cargo.toml" >&2
  exit 1
fi

if [[ -z "${version}" ]]; then
  echo "error: failed to read package version from Cargo.toml" >&2
  exit 1
fi

archive_name="${package_name}-${version}.tar.gz"
archive_path="${output_dir}/${archive_name}"
pkgbuild_path="${output_dir}/PKGBUILD"

mkdir -p "${output_dir}"
rm -f "${archive_path}" "${pkgbuild_path}"

git -C "${repo_root}" ls-files --cached --others --exclude-standard -z \
  | tar \
    --null \
    --directory="${repo_root}" \
    --transform="s,^,${package_name}-${version}/," \
    --files-from=- \
    --create \
    --gzip \
    --file "${archive_path}"

sha256=$(
  sha256sum "${archive_path}" | awk '{print $1}'
)

sed \
  -e "s/@VERSION@/${version}/g" \
  -e "s/@SHA256@/${sha256}/g" \
  "${template_path}" > "${pkgbuild_path}"

echo "Wrote ${archive_path}"
echo "Wrote ${pkgbuild_path}"
echo
echo "Next steps:"
echo "  cd ${output_dir}"
echo "  makepkg -f"
