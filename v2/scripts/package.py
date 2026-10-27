"""Deterministic development ZIP and closed member inventory; never installs."""
from qa_env import *
import hashlib,json,zipfile,io,stat,platform
sha=lambda b:hashlib.sha256(b).hexdigest()
def package():
    fresh=json.loads((QA/'fresh-build.json').read_text());assert fresh['exitCode']==0
    for name,digest in fresh['sourceHashes'].items():assert sha((V2/name).read_bytes())==digest,'fresh build is stale: '+name
    binary=pathlib.Path(fresh['binary']);assert sha(binary.read_bytes())==fresh['binarySha256']
    assert platform.system()=='Darwin'and platform.machine()=='arm64','ARM64 development package requires actual ARM64 build host'
    files={'bin/modify-hash-v2':binary.read_bytes(),'resources/hasher.png':(V2/'resources/hasher.png').read_bytes(),'LICENSE':(V2/'LICENSE').read_bytes(),'NOTICE':(V2/'NOTICE').read_bytes()}
    for p in (V2/'licenses').rglob('*'):
        if p.is_file():files['resources/'+str(p.relative_to(V2))]=p.read_bytes()
    files['resources/dependency-licenses.json']=(V2/'evidence/dependency-licenses.json').read_bytes()
    manifest=json.loads((V2/'extension.json').read_text());assert manifest['artifacts']=={};assert all(v['state']=='not-run'for v in manifest['verification'])
    manifest['files']={n:sha(b)for n,b in sorted(files.items())}
    files['extension.json']=(json.dumps(manifest,sort_keys=True,indent=2)+'\n').encode()
    def make():
        buffer=io.BytesIO()
        with zipfile.ZipFile(buffer,'w',compression=zipfile.ZIP_STORED)as archive:
            for name,data in sorted(files.items()):
                assert not name.startswith('/')and'..'not in pathlib.PurePosixPath(name).parts
                item=zipfile.ZipInfo(name,(1980,1,1,0,0,0));item.create_system=3;item.external_attr=(stat.S_IFREG|(0o755 if name.startswith('bin/')else 0o644))<<16
                archive.writestr(item,data)
        return buffer.getvalue()
    data=make();assert data==make(),'nondeterministic ZIP'
    dist=V2/'dist';dist.mkdir(exist_ok=True);out=dist/'modify-hash-v2-0.2.0-aarch64-apple-darwin-dev.zip';out.write_bytes(data)
    extracted=QA/('extracted-'+sha(data)[:16]);extracted.mkdir(exist_ok=True)
    with zipfile.ZipFile(io.BytesIO(data))as z:
        assert set(z.namelist())==set(files)and len(z.namelist())==len(files)
        for item in z.infolist():
            assert stat.S_ISREG(item.external_attr>>16)
            payload=z.read(item.filename);assert sha(payload)==sha(files[item.filename])
            dest=extracted/item.filename;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(payload);dest.chmod((item.external_attr>>16)&0o777)
    actual={str(p.relative_to(extracted)):sha(p.read_bytes())for p in extracted.rglob('*')if p.is_file()}
    assert actual=={n:sha(b)for n,b in files.items()}
    assert {n:h for n,h in actual.items()if n!='extension.json'}==manifest['files']
    exe=extracted/'bin/modify-hash-v2'
    info=json.loads(subprocess.check_output([str(exe),'--protocol-info'],cwd=QA,env=environment()));assert info['id']==manifest['id']and info['protocol']==1 and set(info['commands'])==set(c['name']for c in manifest['commands'])
    report={'archive':str(out),'sha256':sha(data),'executableSha256':sha(exe.read_bytes()),'members':actual,'memberCount':len(actual),'extracted':str(extracted),'binary':str(exe),'deterministicRepack':True,'protocolInfo':info,'artifacts':{},'installed':False}
    (QA/'package-report.json').write_text(json.dumps(report,indent=2)+'\n')
    print('PASS deterministic closed package',len(actual),'members',sha(data));return report
if __name__=='__main__':package()
