from qa_env import *
import json,hashlib,re
meta=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1'],cwd=V2,env=environment()))
pins={}
for block in (V2/'Cargo.lock').read_text().split('[[package]]')[1:]:
    record=dict(re.findall(r'^([A-Za-z_]+) = "([^"]*)"$',block,re.M));pins[(record['name'],record['version'])]=record
out=V2/'licenses';out.mkdir(exist_ok=True);inventory=[]
for p in sorted(meta['packages'],key=lambda p:(p['name'],p['version'])):
    root=pathlib.Path(p['manifest_path']).parent
    if root==V2:continue
    names=[]
    for f in root.iterdir():
        if f.is_file() and any(x in f.name.lower() for x in ['license','licence','copying','copyright','notice','authors']):names.append(f)
        if f.is_dir() and f.name.lower()in ['licenses','license']:
            names.extend(x for x in f.rglob('*')if x.is_file())
    if p.get('license_file'):
        f=root/p['license_file']
        if f.is_file():names.append(f)
    row={'name':p['name'],'version':p['version'],'declaredLicense':p.get('license'),'source':p.get('source'),'registryChecksum':pins[(p['name'],p['version'])].get('checksum'),'files':{}}
    for f in sorted(set(names)):
        rel=f.relative_to(root);dest=out/(p['name']+'-'+p['version'])/rel;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(f.read_bytes());row['files'][str(dest.relative_to(V2))]=hashlib.sha256(f.read_bytes()).hexdigest()
    row['evidenceGap']=not bool(row['files']);inventory.append(row)
# Classification translation is derived from this exact locked MIT source.
for name in ['file-type','adm-zip']:
    root=QA/'sourcecopy/node_modules'/name;p=json.loads((root/'package.json').read_text());row={'name':name,'version':p['version'],'declaredLicense':p['license'],'source':'unchanged V1 package-lock.json; QA only except file-type category translation','files':{}}
    for f in root.iterdir():
        if f.is_file()and any(x in f.name.lower()for x in ['license','licence','copying','copyright','notice','authors']):
            dest=out/(name+'-'+p['version'])/f.name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(f.read_bytes());row['files'][str(dest.relative_to(V2))]=hashlib.sha256(f.read_bytes()).hexdigest()
    row['evidenceGap']=not bool(row['files']);inventory.append(row)
(V2/'evidence/dependency-licenses.json').write_text(json.dumps(inventory,indent=2)+'\n')
print('license packages',len(inventory),'gaps',[p['name']for p in inventory if p['evidenceGap']])
