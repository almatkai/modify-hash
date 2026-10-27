from qa_env import *
import json, concurrent.futures
# Read-only rustup toolchains; all Cargo caches and targets are disposable and owned.
def gate(name,args,env=None):return name,run(args,name,env=env,timeout=300)
results={}
for name,args in [
 ('fmt',['cargo','fmt','--check']),
 ('stable-test',['cargo','test','--locked','--all-targets']),
 ('stable-clippy',['cargo','clippy','--locked','--all-targets','--','-D','warnings']),
 ('msrv-test',['cargo','+1.85.0','test','--locked','--all-targets']),
 ('msrv-clippy',['cargo','+1.85.0','clippy','--locked','--all-targets','--','-D','warnings']),
 ('windows-check',['cargo','check','--locked','--all-targets','--target','x86_64-pc-windows-gnu']),
 ('windows-clippy',['cargo','clippy','--locked','--all-targets','--target','x86_64-pc-windows-gnu','--','-D','warnings']),
 ('release',['cargo','build','--locked','--release']),
]:
 results[name]=run(args,name,timeout=300)
(QA/'gates.json').write_text(json.dumps(results,indent=2)+'\n')
raise SystemExit(any(results.values()))
