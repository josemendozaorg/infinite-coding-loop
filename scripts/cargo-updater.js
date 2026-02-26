module.exports = {
  readVersion: function (contents) {
    const match = contents.match(/^version\s*=\s*"([^"]+)"/m);
    if (!match) {
      throw new Error('Could not find version in Cargo.toml');
    }
    return match[1];
  },
  writeVersion: function (contents, version) {
    return contents.replace(/^version\s*=\s*"[^"]+"/m, `version = "${version}"`);
  }
};
