"""Required isolated gates. No commit/push/install/host mutation."""
from qa_env import *
import json
# Check existing toolchains before rustup proxies can try acquiring a missing one.
installed=subprocess.check_output(['rustup','toolchain','list'],env=environment(),text=True)
assert 'stable-'in installed and '1.85.0-'in installed,'Required stable/1.85.0 toolchains are absent; install nothing automatically'
targets=subprocess.check_output(['rustup','target','list','--installed'],env=environment(),text=True)
assert 'x86_64-pc-windows-gnu'in targets,'Existing Windows GNU target is required; no tool/target installation is performed'
steps=[('source',['python3',str(V2/'scripts/source_qa.py')]),('host-patch',['python3',str(V2/'scripts/host_patch.py')]),('renderer',['node',str(V2/'scripts/renderer_qa.cjs')]),('schema',['python3',str(V2/'scripts/schema_qa.py')]),('rust',['python3',str(V2/'scripts/gates.py')]),('licenses',['python3',str(V2/'scripts/licenses.py')]),('fresh',['python3',str(V2/'scripts/fresh_build.py')])]
results={}
for name,args in steps:
    results[name]=run(args,'verify-'+name,timeout=600)
    if results[name]:break
(QA/'verify-results.json').write_text(json.dumps(results,indent=2)+'\n')
raise SystemExit(any(results.values())or len(results)!=len(steps))
