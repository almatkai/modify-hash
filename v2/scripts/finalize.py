from qa_env import *
import json,hashlib,sys,re
from package import package
sha=lambda b:hashlib.sha256(b).hexdigest()
def hashes():
    listed=subprocess.check_output(['git','ls-files','-z','--cached','--others','--exclude-standard'],cwd=V2.parent).split(b'\0')
    records={}
    for raw in listed:
        if not raw:continue
        name=raw.decode();p=V2.parent/name
        if name=='v2/FILE_HASHES.json'or not(name=='README.md'or name.startswith('v2/')):continue
        if p.is_file():records[name]=sha(p.read_bytes())
    (V2/'FILE_HASHES.json').write_text(json.dumps({'scope':'All Git-visible v2 files plus root README; excludes this index and historical V1; working bytes, rehash after staging','files':dict(sorted(records.items()))},indent=2)+'\n')
    print('parent review index',len(records),'files',sha((V2/'FILE_HASHES.json').read_bytes()))
def main():
    if '--hash-only'in sys.argv:hashes();return
    required=json.loads((QA/'verify-results.json').read_text());assert len(required)==7 and not any(required.values()),'run verify.py first'
    report=package()
    assert run(['node',str(V2/'scripts/renderer_qa.cjs'),report['binary']],'extracted-renderer',timeout=60)==0,'extracted-renderer'
    for name,script in [('extracted-process','process_qa.py'),('extracted-extended','extended_qa.py')]:
        assert run(['python3',str(V2/'scripts'/script),report['binary']],name,timeout=180)==0,name
    # Copy review evidence only after extracted executable gates, then inventory it.
    names=['gates.json','source-results.json','oracle-results.json','renderer-results.json','schema-results.json','process-results.json','extended-results.json','fixture-original-hashes.json','extended-fixture-original-hashes.json','fresh-build.json','package-report.json','verify-results.json']
    for name in names:(V2/'evidence'/name).write_bytes((QA/name).read_bytes())
    assert not any(r['evidenceGap']for r in json.loads((V2/'evidence/dependency-licenses.json').read_text()))
    original=json.loads((V2/'evidence/objects.json').read_text())
    for name,record in original.items():
        if name.startswith('v1/')and name!='v1/README.md':assert sha((V2.parent/name[3:]).read_bytes())==record['sha256'],name
    assert not subprocess.check_output(['git','diff','--name-only'],cwd=V2.parent,text=True).splitlines() or subprocess.check_output(['git','diff','--name-only'],cwd=V2.parent,text=True).splitlines()==['README.md']
    for name in ['fresh-locked-https-sdk.log','windows-link-probe.log','source-oracle.log']:
        if (QA/name).exists():(V2/'evidence'/name).write_bytes((QA/name).read_bytes())
    validation=V2/'VALIDATION.md'
    if validation.exists():
        text=validation.read_text()
        text=re.sub(r'ZIP SHA256 `[0-9a-f]{64}`', 'ZIP SHA256 `'+report['sha256']+'`',text)
        text=re.sub(r'Executable SHA256 `[0-9a-f]{64}`', 'Executable SHA256 `'+report['executableSha256']+'`',text)
        count=json.loads((QA/'process-results.json').read_text())['cancelledBatchCommitted']
        text=re.sub(r'Cancellation left [0-9]+ committed file', 'Cancellation left '+str(count)+' committed file',text)
        validation.write_text(text)
    hashes()
if __name__=='__main__':main()
