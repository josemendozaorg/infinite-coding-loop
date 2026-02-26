module.exports = {
  readVersion: function (contents) {
    const match = contents.match(/pulpo-engine\s*=\s*\{\s*version\s*=\s*"([^"]+)"/m);
    if (!match) {
      throw new Error('Could not find pulpo-engine version in root Cargo.toml');
    }
    return match[1];
  },
  writeVersion: function (contents, version) {
    return contents.replace(/(pulpo-engine\s*=\s*\{\s*version\s*=\s*")[^"]+(")/m, `$1${version}$2`);
  }
};
