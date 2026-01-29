const fs = require('fs');
const path = require('path');

const inputPath = process.argv[2];
if (!inputPath) {
    console.error("Usage: node viz_schema.js <path_to_schema.json>");
    process.exit(1);
}

const baseDir = path.dirname(inputPath);
const schema = JSON.parse(fs.readFileSync(inputPath, 'utf8'));

console.log("classDiagram");

// Helper to process a definition object
function processDefinition(name, def) {
    console.log(`    class ${name} {`);

    // Properties
    if (def.properties) {
        for (const [propName, propDef] of Object.entries(def.properties)) {
            let type = propDef.type || "Any";
            if (propDef.enum) type = "Enum";
            console.log(`        +${type} ${propName}`);
        }
    }

    // Handle AllOf (Merge properties from refs potentially)
    if (def.allOf) {
        def.allOf.forEach(sub => {
            if (sub.properties) {
                for (const [propName, propDef] of Object.entries(sub.properties)) {
                    let type = propDef.type || "Any";
                    if (propDef.enum) type = "Enum";
                    console.log(`        +${type} ${propName}`);
                }
            }
        });
    }

    console.log(`    }`);

    // Annotations
    if (def.description) {
        const desc = def.description.replace(/"/g, "'").substring(0, 50) + "...";
        console.log(`    note for ${name} "${desc}"`);
    }
}

// 1. Process Internal Definitions
const defs = schema.definitions || schema.$defs || {};
for (const [name, def] of Object.entries(defs)) {
    processDefinition(name, def);
}

// 2. Process External Refs in oneOf
if (schema.oneOf) {
    schema.oneOf.forEach(ref => {
        if (ref.$ref && ref.$ref.startsWith("./")) {
            // Resolve external file
            const refPath = path.join(baseDir, ref.$ref);
            if (fs.existsSync(refPath)) {
                const subSchema = JSON.parse(fs.readFileSync(refPath, 'utf8'));
                // Use title as Class Name
                const className = subSchema.title || path.basename(refPath, ".schema.json");

                // Extract main definition from subSchema (usually inside allOf or properties directly)
                // In our format, it's often allOf[1] -> properties
                // We'll simplisticly treat subSchema as the def
                processDefinition(className, subSchema);

                // Add inheritance link to EntityMetadata if it refs it
                // We assume implicit inheritance for visualization sake or check refs
                if (subSchema.allOf) {
                    subSchema.allOf.forEach(sub => {
                        if (sub.$ref && sub.$ref.includes("EntityMetadata")) {
                            console.log(`    EntityMetadata <|-- ${className}`);
                        }
                    });
                }
            }
        }
    });
}

// 3. Inheritance mappings within internal defs
for (const [name, def] of Object.entries(defs)) {
    if (def.allOf) {
        def.allOf.forEach(sub => {
            if (sub.$ref) {
                const parentName = sub.$ref.split('/').pop();
                console.log(`    ${parentName} <|-- ${name}`);
            }
        });
    }
}

// 4. Special Case: GraphRules (Metamodel)
if (defs.GraphRules && defs.GraphRules.rules) {
    console.log("graph TD");
    defs.GraphRules.rules.forEach(rule => {
        const src = rule.source.replace("Entity", "");
        const tgt = rule.target.replace("Entity", "");
        console.log(`    ${src} -->|${rule.relation}| ${tgt}`);
    });
}
