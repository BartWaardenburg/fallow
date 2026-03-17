// Verify the correct platform-specific package was installed
const PLATFORMS = {
  'darwin-arm64': ['@fallow-cli/darwin-arm64'],
  'darwin-x64': ['@fallow-cli/darwin-x64'],
  'linux-x64': ['@fallow-cli/linux-x64-gnu', '@fallow-cli/linux-x64-musl'],
  'linux-arm64': ['@fallow-cli/linux-arm64-gnu', '@fallow-cli/linux-arm64-musl'],
  'win32-x64': ['@fallow-cli/win32-x64-msvc'],
};

const platformKey = `${process.platform}-${process.arch}`;
const candidates = PLATFORMS[platformKey];

if (!candidates) {
  console.warn(
    `fallow: No prebuilt binary for ${platformKey}. ` +
    `You can build from source: https://github.com/bartwaardenburg/fallow`
  );
  process.exit(0);
}

const found = candidates.some((pkg) => {
  try {
    require.resolve(`${pkg}/package.json`);
    return true;
  } catch {
    return false;
  }
});

if (!found) {
  console.warn(
    `fallow: No platform package installed for ${platformKey}. ` +
    `This may happen if you used --no-optional. ` +
    `Run 'npm install' to fix.`
  );
}
