const cargoUpdater = require('./scripts/cargo-updater.js');
const rootCargoUpdater = require('./scripts/root-cargo-updater.js');

module.exports = {
  bumpFiles: [
    { filename: 'package.json', type: 'json' },
    { filename: 'apps/pulpo-visualizer/package.json', type: 'json' },
    { filename: 'packages/pulpo-schema/package.json', type: 'json' },
    { filename: 'pulpo-ontologies/software-engineering/package.json', type: 'json' },
    { filename: 'apps/pulpo-cli/Cargo.toml', updater: cargoUpdater },
    { filename: 'apps/pulpo-tui/Cargo.toml', updater: cargoUpdater },
    { filename: 'packages/pulpo-engine/Cargo.toml', updater: cargoUpdater },
    { filename: 'packages/pulpo-tools/Cargo.toml', updater: cargoUpdater },
    { filename: 'tests/pulpo-e2e/Cargo.toml', updater: cargoUpdater },
    { filename: 'Cargo.toml', updater: rootCargoUpdater }
  ],
  packageFiles: [
    { filename: 'package.json', type: 'json' }
  ]
};
