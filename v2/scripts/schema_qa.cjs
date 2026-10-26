// Convenience entry point; the evaluator has no installed package dependency.
require('node:child_process').execFileSync('python3',[require('node:path').join(__dirname,'schema_qa.py')],{stdio:'inherit'});
