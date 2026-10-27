"""All test output/environment paths are contained in this clone's ignored v2/.qa."""
import os, pathlib, subprocess, signal, sys
sys.dont_write_bytecode=True
V2=pathlib.Path(__file__).resolve().parents[1]
QA=V2/'.qa'
def environment(cargo='cargo-home'):
    env=dict(os.environ)
    env['RUSTUP_HOME']=os.environ.get('RUSTUP_HOME',str(pathlib.Path.home()/'.rustup'))
    for key,name in [('HOME','home'),('TMPDIR','tmp'),('TMP','tmp'),('TEMP','tmp'),('XDG_CACHE_HOME','cache'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('CARGO_HOME',cargo),('CARGO_TARGET_DIR','target-isolated'),('npm_config_cache','npm-cache')]:
        p=QA/name;p.mkdir(parents=True,exist_ok=True);env[key]=str(p)
    env['CARGO_HTTP_TIMEOUT']='30';env['CARGO_NET_RETRY']='1'
    return env
def run(args,name,cwd=V2,env=None,timeout=300):
    p=subprocess.Popen(args,cwd=cwd,env=env or environment(),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,start_new_session=True)
    try: output=p.communicate(timeout=timeout)[0]
    except subprocess.TimeoutExpired:
        os.killpg(p.pid,signal.SIGTERM)
        try: output=p.communicate(timeout=5)[0]
        except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);output=p.communicate()[0]
    (QA/(name+'.log')).write_bytes(output)
    print(name,p.returncode,output.decode(errors='replace')[-1600:],flush=True)
    return p.returncode
