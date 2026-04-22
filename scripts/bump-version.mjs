import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const rootDir = process.cwd();
const packageJsonPath = resolve(rootDir, 'package.json');
const cargoTomlPath = resolve(rootDir, 'src-tauri', 'Cargo.toml');
const tauriConfigPath = resolve(rootDir, 'src-tauri', 'tauri.conf.json');

const args = process.argv.slice(2);

function fail(message) {
  console.error(message);
  process.exit(1);
}

function parseVersion(value) {
  const normalized = String(value ?? '').trim();
  const match = normalized.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!match) {
    fail(`Unsupported version: ${normalized}`);
  }

  return {
    raw: normalized,
    major: Number.parseInt(match[1], 10),
    minor: Number.parseInt(match[2], 10),
    patch: Number.parseInt(match[3], 10),
  };
}

function formatVersion(version) {
  return `${version.major}.${version.minor}.${version.patch}`;
}

function bumpPatch(version) {
  return {
    major: version.major,
    minor: version.minor,
    patch: version.patch + 1,
  };
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function readCargoVersion() {
  const content = readFileSync(cargoTomlPath, 'utf8');
  const lines = content.split('\n');
  let inPackageSection = false;

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed === '[package]') {
      inPackageSection = true;
      continue;
    }
    if (inPackageSection && trimmed.startsWith('[')) {
      break;
    }
    if (inPackageSection) {
      const match = line.match(/^\s*version\s*=\s*"([^"]+)"\s*$/);
      if (match) {
        return match[1];
      }
    }
  }

  fail(`Unable to find package version in ${cargoTomlPath}`);
}

function writeCargoVersion(nextVersion) {
  const content = readFileSync(cargoTomlPath, 'utf8');
  const lines = content.split('\n');
  let inPackageSection = false;
  let replaced = false;

  const updatedLines = lines.map((line) => {
    const trimmed = line.trim();
    if (trimmed === '[package]') {
      inPackageSection = true;
      return line;
    }
    if (inPackageSection && trimmed.startsWith('[')) {
      inPackageSection = false;
      return line;
    }
    if (inPackageSection && /^\s*version\s*=\s*"([^"]+)"\s*$/.test(line)) {
      replaced = true;
      return line.replace(/(^\s*version\s*=\s*")([^"]+)("\s*$)/, `$1${nextVersion}$3`);
    }
    return line;
  });

  if (!replaced) {
    fail(`Unable to update package version in ${cargoTomlPath}`);
  }

  writeFileSync(cargoTomlPath, updatedLines.join('\n'), 'utf8');
}

function readAllVersions() {
  const packageJson = readJson(packageJsonPath);
  const tauriConfig = readJson(tauriConfigPath);

  const versions = {
    packageJson: String(packageJson.version ?? '').trim(),
    cargoToml: readCargoVersion(),
    tauriConfig: String(tauriConfig.version ?? '').trim(),
  };

  const uniqueVersions = new Set(Object.values(versions));
  if (uniqueVersions.size !== 1) {
    fail(
      `Version mismatch detected: package.json=${versions.packageJson}, src-tauri/Cargo.toml=${versions.cargoToml}, src-tauri/tauri.conf.json=${versions.tauriConfig}`
    );
  }

  parseVersion(versions.packageJson);
  return versions.packageJson;
}

function setVersion(nextVersion) {
  parseVersion(nextVersion);

  const packageJson = readJson(packageJsonPath);
  packageJson.version = nextVersion;
  writeJson(packageJsonPath, packageJson);

  const tauriConfig = readJson(tauriConfigPath);
  tauriConfig.version = nextVersion;
  writeJson(tauriConfigPath, tauriConfig);

  writeCargoVersion(nextVersion);
}

const currentVersion = readAllVersions();
const nextPatchVersion = formatVersion(bumpPatch(parseVersion(currentVersion)));

if (args.includes('--print-current')) {
  console.log(currentVersion);
  process.exit(0);
}

if (args.includes('--print-next-patch')) {
  console.log(nextPatchVersion);
  process.exit(0);
}

if (args.includes('--set-next-patch')) {
  setVersion(nextPatchVersion);
  console.log(nextPatchVersion);
  process.exit(0);
}

const setIndex = args.indexOf('--set');
if (setIndex !== -1) {
  const nextVersion = args[setIndex + 1];
  if (!nextVersion) {
    fail('Missing version after --set');
  }
  setVersion(nextVersion);
  console.log(nextVersion);
  process.exit(0);
}

fail('Usage: node scripts/bump-version.mjs [--print-current | --print-next-patch | --set-next-patch | --set <version>]');
