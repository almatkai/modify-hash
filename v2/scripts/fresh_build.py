from qa_env import *
import json,hashlib,uuid
name='fresh-'+uuid.uuid4().hex
env=environment(name+'-cargo-home');env['CARGO_TARGET_DIR']=str(QA/(name+'-target'))
code=run(['cargo','build','--release','--locked'],'fresh-locked-https-sdk',env=env,timeout=300)
binary=pathlib.Path(env['CARGO_TARGET_DIR'])/'release/modify-hash-v2'
record={'exitCode':code,'cargoHome':env['CARGO_HOME'],'targetDir':env['CARGO_TARGET_DIR'],'binary':str(binary),'sdk':'https://github.com/almatkai/tezbar-extension-sdk-rust','revision':'e955911f0bf5b558f9d3eb5183df0adc95c22af2','locked':True,'sourceHashes':{str(p.relative_to(V2)):hashlib.sha256(p.read_bytes()).hexdigest()for p in [V2/'Cargo.toml',V2/'Cargo.lock',*(V2/'src').glob('*.rs')]}}
if code==0:record['binarySha256']=hashlib.sha256(binary.read_bytes()).hexdigest()
(QA/'fresh-build.json').write_text(json.dumps(record,indent=2)+'\n')
raise SystemExit(code)
