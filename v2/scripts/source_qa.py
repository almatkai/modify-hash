from qa_env import *
import json,hashlib
source=QA/'sourcecopy';env=environment()
# Recreate unchanged baseline only when absent; npm lock stays byte-identical.
if not (source/'package.json').exists():
    files=subprocess.check_output(['git','ls-tree','-r','--name-only','3fdbb4822b6d9892bfbb685094a3e8a7b69270f0'],cwd=V2.parent,text=True).splitlines()
    for f in files:
        p=source/f;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(subprocess.check_output(['git','show','3fdbb4822b6d9892bfbb685094a3e8a7b69270f0:'+f],cwd=V2.parent))
results={}
results['npm-ci']=run(['npm','ci','--ignore-scripts','--no-audit','--no-fund'],'source-npm-ci',cwd=source,timeout=120)
results['test']=run(['npm','test'],'original-test',cwd=source)
results['typecheck']=run(['npm','run','typecheck'],'original-typecheck',cwd=source)
assert (source/'package-lock.json').read_bytes()==(V2.parent/'package-lock.json').read_bytes()
entry=QA/'oracle-entry.ts';entry.write_text(''.join(f'export {{ default as {c} }} from "./sourcecopy/src/{c}";\n' for c in ['modifyHash','restoreHash','zipCompress','zipExtract'])+'export {fileTypeFromFile} from "./sourcecopy/node_modules/file-type/index.js";\nexport {default as Zip} from "./sourcecopy/node_modules/adm-zip/adm-zip.js";\n')
results['bundle']=run([str(source/'node_modules/.bin/esbuild'),str(entry),'--bundle','--platform=node','--format=cjs','--external:react','--external:react/jsx-runtime','--external:@raycast/api','--outfile='+str(QA/'oracle-bundle.cjs')],'oracle-bundle')
results['oracle']=run(['node',str(V2/'scripts/oracle.cjs')],'source-oracle')
(QA/'source-results.json').write_text(json.dumps(results,indent=2)+'\n')
raise SystemExit(results['npm-ci'] or results['bundle'] or results['oracle'])
