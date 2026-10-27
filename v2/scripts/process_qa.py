"""Production SDK JSONL-pipe tests; never touch live host services or user paths."""
from qa_env import *
import base64, hashlib, json, queue, threading, time, uuid, zipfile, io, sys
MARK=b'#$1024$#'
ROOT=QA/'fixtures'/uuid.uuid4().hex;ROOT.mkdir(parents=True,exist_ok=True)
MP4=b'\0\0\0\x18ftypisom'+b'\0'*20
IMAGE=b'GIF89a'+b'\0'*40
AUDIO=b'fLaC'+b'\0'*40
BINARY=pathlib.Path(sys.argv[1]) if len(sys.argv)>1 else QA/'target-isolated/release/modify-hash-v2'
counts={}; hashes={}
def check(name,condition=True):
    assert condition,name
    counts[name]=counts.get(name,0)+1
class Pipe:
    def __init__(self):
        self.p=subprocess.Popen([str(BINARY)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=environment(),cwd=QA)
        self.q=queue.Queue();self.frames=[];self.pending={};self.reverse=[]
        def read():
            while True:
                line=self.p.stdout.readline(16*1024*1024+2)
                if not line:self.q.put(None);return
                assert len(line)<=16*1024*1024+1
                self.q.put(json.loads(line))
        self.reader=threading.Thread(target=read,daemon=True);self.reader.start()
        self.stderr=threading.Thread(target=lambda:self.p.stderr.read(1024*1024),daemon=True);self.stderr.start()
        hello=self.q.get(timeout=5);check('hello',hello['id']=='com.tezbar.native.modify-hash' and hello['protocol']==1)
        for name in ['package','data']:(ROOT/name).mkdir(exist_ok=True)
        r=self.call('initialize',{'protocol':1,'extensionId':hello['id'],'platform':'macOS','packageDir':str(ROOT/'package'),'dataDir':str(ROOT/'data'),'tempDir':str(ROOT/'unused-sdk-temp'),'grantedCapabilities':['selection.read'],'migrationPaths':{}})
        check('initialize',r['ready'])
    def send(self,v):self.p.stdin.write(json.dumps(v,separators=(',',':')).encode()+b'\n');self.p.stdin.flush()
    def start(self,method,params):
        rid='h:'+str(uuid.uuid4());self.send({'type':'request','id':rid,'method':method,'params':params});return rid
    def wait(self,rid,reply=None,timeout=20,allow_error=False):
        end=time.monotonic()+timeout
        while rid not in self.pending:
            f=self.q.get(timeout=max(.01,end-time.monotonic()));assert f is not None,'Unexpected process EOF'
            self.frames.append(f)
            if f['type']=='response':self.pending[f['id']]=f
            elif f['type']=='request':
                self.reverse.append(f)
                assert f['method']=='host.selection.read' and f['params']=={'kind':'files'},f
                if reply is not None:self.send({'type':'response','id':f['id'],'result':reply})
            assert time.monotonic()<end
        f=self.pending.pop(rid)
        if 'error' in f:
            if allow_error:return f
            raise AssertionError(f)
        return f['result']
    def call(self,m,p,**kw):return self.wait(self.start(m,p),**kw)
    def open(self,cmd='modifyHash',prefs=None):
        sid=str(uuid.uuid4());v=self.call('session.open',{'sessionId':sid,'commandName':cmd,'preferences':prefs or {},'argumentValues':{}});return sid,v
    def action(self,sid,aid,values=None,**kw):
        p={'sessionId':sid,'operation':'extension:invoke-action','payload':{'actionId':aid}}
        if values is not None:p['payload']['formValues']=values
        return self.call('session.call',p,**kw)
    def dispose(self,sid):return self.call('session.dispose',{'sessionId':sid})
    def close(self):
        self.call('shutdown',{});self.p.stdin.close();self.p.wait(timeout=5);self.reader.join(1);self.stderr.join(1);check('shutdown',self.p.returncode==0)
def form(v):return {n['props']['id']:n['props']['value'] for n in v['root']['children'] if 'id'in n.get('props',{})}
def action(v,kind):return next(a['id'] for a in v['actions'] if a['kind']==kind)
def results(v):return json.loads('\n'.join(x[4:] for x in v['root']['props']['markdown'].splitlines()))
def prepare(p,cmd,paths,prefs=None):
    sid,v=p.open(cmd,prefs);f=form(v);f['paths']=json.dumps([str(x) for x in paths]);f['_draftRevision']=1
    field=next(n['props']['actionId'] for n in v['root']['children'] if n.get('props',{}).get('id')=='paths')
    v=p.action(sid,field,f);return sid,v,f,field
def execute(p,cmd,paths,prefs=None):
    for path in paths:
        path=pathlib.Path(path)
        assert str(path.absolute()).startswith(str(QA)),path
        if path.is_symlink():continue
        for candidate in ([path] if path.is_file() else path.rglob('*') if path.is_dir() else []):
            if candidate.is_file() and not candidate.is_symlink():hashes.setdefault(str(candidate.relative_to(QA)),hashlib.sha256(candidate.read_bytes()).hexdigest())
    sid,v,f,_=prepare(p,cmd,paths,prefs);aid=action(v,'submit-form');v=p.action(sid,aid,f);r=results(v);return sid,aid,r

def write(name,data):
    p=ROOT/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data);hashes[str(p.relative_to(QA))]=hashlib.sha256(data).hexdigest();return p

def main():
    p=Pipe()
    a=write('basic/雪 strange name.noextension',MP4)
    sid,aid,r=execute(p,'modifyHash',[a]);check('append-exact',a.read_bytes()==MP4+MARK);check('md5-logged',r['results'][0]['new_md5']==hashlib.md5(MP4+MARK).hexdigest())
    before=a.read_bytes();v=p.action(sid,aid,{'bad':'snapshot'});check('consumed-action-no-replay',a.read_bytes()==before and results(v)==r)
    p.dispose(sid)
    sid,_,r=execute(p,'modifyHash',[a],{'showMD5Log':False});check('double-marker',a.read_bytes()==MP4+MARK*2);check('md5-disabled',r['results'][0]['old_md5'] is None);p.dispose(sid)
    for n in [1,0]:
        sid,_,_=execute(p,'restoreHash',[a]);check('restore-one-marker',a.read_bytes()==MP4+MARK*n);p.dispose(sid)
    original=MP4+b'unrecognized'+MARK[:-1];a.write_bytes(original);sid,_,r=execute(p,'restoreHash',[a]);check('restore-preserves-unrecognized',a.read_bytes()==original and r['results'][0]['state']=='unchanged' and r['results'][0]['old_md5']==hashlib.md5(original).hexdigest() and r['results'][0]['new_md5']==hashlib.md5(original).hexdigest());p.dispose(sid)
    image=write('filters/photo.mp4',IMAGE);audio=write('filters/music.jpg',AUDIO);text=write('filters/not-media.mp4',b'plain bytes')
    sid,_,r=execute(p,'modifyHash',[image,audio,text]);check('default-media-filters',image.read_bytes()==IMAGE and audio.read_bytes()==AUDIO and text.read_bytes()==b'plain bytes');p.dispose(sid)
    sid,_,r=execute(p,'modifyHash',[image,audio],{'enableImage':True,'enableAudio':True});check('content-not-extension',image.read_bytes()==IMAGE+MARK and audio.read_bytes()==AUDIO+MARK);p.dispose(sid)
    # Every field participates in monotonically versioned snapshots. Old A must
    # stay inert after B, invalid intermediate data, and a later identical A.
    a=write('draft/a',MP4);b=write('draft/b',MP4)
    sid,v,f,field=prepare(p,'modifyHash',[a]);old=action(v,'submit-form')
    for rev,paths in [(2,[str(b)]),(3,'invalid'),(4,[str(a)])]:
        f=dict(f,_draftRevision=rev,paths=json.dumps(paths) if isinstance(paths,list) else paths);v=p.action(sid,field,f)
    latest=action(v,'submit-form');p.action(sid,old,f);check('A-B-invalid-A-fencing',a.read_bytes()==MP4 and b.read_bytes()==MP4)
    changed=dict(f,showMD5Log=False,_draftRevision=5);v=p.action(sid,field,changed);p.action(sid,latest,f);check('all-fields-fencing',a.read_bytes()==MP4);p.dispose(sid)
    # A malformed submission consumes its token even when SDK staged state rolls back.
    sid,v,f,_=prepare(p,'modifyHash',[a]);aid=action(v,'submit-form');p.action(sid,aid,{});p.action(sid,aid,f);check('malformed-submit-consumed',a.read_bytes()==MP4);p.dispose(sid)
    # Refresh, search, unknown IDs, and load-more have no effects.
    sid,v,_,_=prepare(p,'modifyHash',[a]);before=a.read_bytes()
    for op in ['extension:refresh-session','extension:search-text-changed','extension:load-more']:
        p.call('session.call',{'sessionId':sid,'operation':op,'payload':{'searchText':'ignored'}})
    p.action(sid,'queued-stale-id');check('effect-free-non-actions',a.read_bytes()==before and not p.reverse);p.dispose(sid)
    # Explicit selection only, and strict absolute-array reply contracts.
    for reply in [None,True,{},['relative'],[42],[],[str(ROOT/'basic')]]:
        sid,v=p.open();aid=action(v,'custom')
        if reply is None:reply='not-an-array'
        v=p.action(sid,aid,dict(form(v),_draftRevision=0),reply=reply);check('selection-contract',v['root']['type']=='Form');p.dispose(sid)
    # Real serialized SDK: fields queue behind the reverse RPC. Selection may
    # produce a candidate, but must never write fields or advance client ordinals.
    selected=write('selection/finder',MP4);typed=write('selection/typed',MP4)
    for edits in [
        [(1,json.dumps([str(typed)]),False)],
        [(1,'A',True),(2,'B',True),(3,'A',True)],
        [(1,'INVALID',True),(2,json.dumps([str(typed)]),False)],
        [(1,'[]',False)],
    ]:
        sid,v=p.open();initial=dict(form(v),_draftRevision=0)
        field=next(n['props']['actionId'] for n in v['root']['children'] if n.get('props',{}).get('id')=='paths')
        if len(edits)>1:
            initial=dict(initial,paths=json.dumps([str(typed)]),_draftRevision=1)
            v=p.action(sid,field,initial)
            edits=[(i+2,json.dumps([str(typed)]) if i==len(edits)-1 else paths,md5) for i,(_,paths,md5) in enumerate(edits)]
        aid=action(v,'custom')
        rid=p.start('session.call',{'sessionId':sid,'operation':'extension:invoke-action','payload':{'actionId':aid,'formValues':initial}})
        reverse=p.q.get(timeout=5);check('selection-reverse-pending',reverse['method']=='host.selection.read')
        queued=[]
        for rev,paths,md5 in edits:
            f=dict(initial,paths=paths,showMD5Log=md5,_draftRevision=rev)
            queued.append(p.start('session.call',{'sessionId':sid,'operation':'extension:invoke-action','payload':{'actionId':field,'formValues':f}}))
        p.send({'type':'response','id':reverse['id'],'result':[str(selected)]})
        candidate=p.wait(rid);c=candidate['root']['props']['selectionCandidate']
        check('selection-does-not-own-revision',candidate['root']['props']['draftRevision']==initial['_draftRevision'] and form(candidate)==form(v))
        for queued_id in queued:latest=p.wait(queued_id)
        check('pending-selection-keeps-all-draft-edits',dict(form(latest),_draftRevision=latest['root']['props']['draftRevision'])==f)
        check('selection-fields-effect-free',selected.read_bytes()==MP4 and typed.read_bytes()==MP4)
        before=form(latest);stale=p.action(sid,c['acceptActionId'],dict(initial,paths=c['paths'],_draftRevision=1))
        check('late-candidate-inert',form(stale)==before and selected.read_bytes()==MP4)
        # Even an identical old A value must not reopen its old submit token.
        if any(a['kind']=='submit-form' for a in latest['actions']):
            token=action(latest,'submit-form');f=dict(f,_draftRevision=f['_draftRevision']+1,paths='INVALID');p.action(sid,field,f)
            p.action(sid,token,dict(before,_draftRevision=f['_draftRevision']-1));check('selection-stale-submit-inert',typed.read_bytes()==MP4)
        p.dispose(sid)
    # Accept no-edit selection automatically via one-use candidate admission;
    # preserve every nonpath preference and allow repeated deliberate selections.
    sid,v=p.open(prefs={'showMD5Log':False,'enableAudio':True,'enableImage':True,'zipPassword':'synthetic'})
    f=dict(form(v),_draftRevision=0);first_candidate=None
    for number in range(2):
        aid=action(v,'custom');v=p.action(sid,aid,f,reply=[str(selected)])
        c=v['root']['props']['selectionCandidate'];check('selection-candidate-before-submit',not any(a['kind']=='submit-form' for a in v['actions']))
        if first_candidate is None:first_candidate=c
        f=dict(f,paths=c['paths'],_draftRevision=f['_draftRevision']+1)
        v=p.action(sid,c['acceptActionId'],f)
        check('selection-admission-preserves-preferences',dict(form(v),_draftRevision=v['root']['props']['draftRevision'])==f and f['showMD5Log'] is False and f['enableAudio'] is True and f['enableImage'] is True and f['zipPassword']=='synthetic')
        check('selection-admission-no-effect',selected.read_bytes()==MP4)
        current_token=action(v,'submit-form');again=p.action(sid,c['acceptActionId'],f)
        check('candidate-one-use-token-stable',action(again,'submit-form')==current_token)
    p.action(sid,first_candidate['acceptActionId'],dict(f,_draftRevision=1));check('old-selection-after-new-inert',selected.read_bytes()==MP4)
    token=action(v,'submit-form');done=results(p.action(sid,token,f));check('selected-latest-explicit-submit',selected.read_bytes()==MP4+MARK and done['results'][0]['old_md5'] is None)
    p.action(sid,token,f);check('selected-submit-no-replay',selected.read_bytes()==MP4+MARK);p.dispose(sid)
    selected.write_bytes(MP4)
    # Candidate acceptance cannot smuggle a preference edit or reuse its token
    # after rejection. Empty selection still permits a new explicit selection.
    sid,v=p.open();f=dict(form(v),_draftRevision=0)
    v=p.action(sid,action(v,'custom'),f,reply=[str(selected)]);c=v['root']['props']['selectionCandidate']
    wrong=dict(f,paths=c['paths'],showMD5Log=False,_draftRevision=1)
    v=p.action(sid,c['acceptActionId'],wrong);check('candidate-rejects-preference-smuggling',form(v)['paths']=='[]' and not any(a['kind']=='submit-form' for a in v['actions']))
    v=p.action(sid,c['acceptActionId'],dict(f,paths=c['paths'],_draftRevision=1));check('rejected-candidate-consumed',form(v)['paths']=='[]')
    v=p.action(sid,action(v,'custom'),f,reply=[]);c=v['root']['props']['selectionCandidate']
    f=dict(f,paths=c['paths'],_draftRevision=1);v=p.action(sid,c['acceptActionId'],f)
    check('empty-selection-editable',not any(a['kind']=='submit-form' for a in v['actions']) and any(a['kind']=='custom' for a in v['actions']))
    v=p.action(sid,action(v,'custom'),f,reply=[str(selected)]);c=v['root']['props']['selectionCandidate']
    v=p.action(sid,c['acceptActionId'],dict(f,paths=c['paths'],_draftRevision=2));check('selection-after-empty-recovers',any(a['kind']=='submit-form' for a in v['actions']) and selected.read_bytes()==MP4);p.dispose(sid)
    # Actual reverse errors/cancellation are read-only and recover without reopen.
    for cancel in [False,True]:
        sid,v,f,field=prepare(p,'modifyHash',[typed]);old=action(v,'submit-form');aid=action(v,'custom')
        rid=p.start('session.call',{'sessionId':sid,'operation':'extension:invoke-action','payload':{'actionId':aid,'formValues':f}})
        reverse=p.q.get(timeout=5);assert reverse['method']=='host.selection.read'
        if cancel:p.send({'type':'cancel','requestId':rid,'reason':'synthetic pending selection cancel'})
        else:p.send({'type':'response','id':reverse['id'],'error':{'code':'synthetic','message':'selection unavailable'}})
        p.wait(rid,allow_error=cancel)
        if cancel:p.send({'type':'response','id':reverse['id'],'result':[str(selected)]})
        v=p.call('session.call',{'sessionId':sid,'operation':'extension:refresh-session','payload':{}})
        check('selection-error-cancel-editable',v['root']['type']=='Form' and form(v)['paths']==f['paths'] and any(a['kind']=='custom' for a in v['actions']))
        p.action(sid,old,f);check('selection-error-old-submit-inert',typed.read_bytes()==MP4)
        f=dict(f,_draftRevision=2,showMD5Log=False);v=p.action(sid,field,f)
        check('selection-error-field-recovers',any(a['kind']=='submit-form' for a in v['actions']))
        p.dispose(sid)
    # Legacy custom producer lacks ownership: fail closed without reverse calls.
    sid,v=p.open();n=len(p.reverse);v=p.action(sid,action(v,'custom'))
    check('selection-needs-snapshot',len(p.reverse)==n and v['root']['type']=='Form');p.dispose(sid)
    # Per-file errors preserve completed successes and do not replay them on retry.
    good=write('partial/good',MP4);ro=write('partial/read-only',MP4);ro.chmod(0o444)
    sid,aid,r=execute(p,'modifyHash',[good,ro,ROOT/'missing']);check('partial-results',good.read_bytes()==MP4+MARK and ro.read_bytes()==MP4 and {x['state'] for x in r['results']}=={'committed','failed'})
    p.action(sid,aid);check('partial-no-replay',good.read_bytes()==MP4+MARK);p.dispose(sid)
    outside=write('outside/untouched',MP4);link=ROOT/'symlink';
    if not link.exists():link.symlink_to(outside)
    sid,_,r=execute(p,'modifyHash',[link]);check('nofollow-leaf',outside.read_bytes()==MP4 and r['results'][0]['state']=='failed');p.dispose(sid)
    dlink=ROOT/'dir-link';
    if not dlink.exists():dlink.symlink_to(ROOT/'outside',target_is_directory=True)
    sid,_,r=execute(p,'modifyHash',[dlink/'untouched']);check('nofollow-ancestor',outside.read_bytes()==MP4 and r['results'][0]['state']=='failed');p.dispose(sid)
    # Real portable ZIP content and naming/collision behavior.
    folder=ROOT/'archive/source';folder.mkdir(parents=True,exist_ok=True);(folder/'empty').mkdir(exist_ok=True);(folder/'one.txt').write_text('one');(folder/'nested').mkdir(exist_ok=True);(folder/'nested/two').write_bytes(MP4)
    sid,aid,r=execute(p,'zipCompress',[folder]);out=pathlib.Path(r['results'][0]['path']);check('zip-created',out.is_file() and out.name=='source.zip')
    with zipfile.ZipFile(out) as z:
        check('zip-member-content',z.read('source/one.txt')==b'one' and z.read('source/nested/two')==MP4 and 'source/empty/'in z.namelist())
    digest=hashlib.sha256(out.read_bytes()).hexdigest();p.action(sid,aid);check('zip-no-recompress-replay',hashlib.sha256(out.read_bytes()).hexdigest()==digest);p.dispose(sid)
    sid,_,r=execute(p,'zipCompress',[folder]);check('zip-output-collision',pathlib.Path(r['results'][0]['path']).name=='source 2.zip');p.dispose(sid)
    sid,aid,r=execute(p,'zipExtract',[out]);extracted=pathlib.Path(r['results'][0]['path']);check('zip-extracted',extracted.name=='source 2' and (extracted/'source/nested/two').read_bytes()==MP4)
    contents=list(ROOT.glob('archive/source*'));p.action(sid,aid);check('extract-no-replay',list(ROOT.glob('archive/source*'))==contents);p.dispose(sid)
    # All archives are synthetic, and all failure outputs remain unpublished.
    attacks=[('traversal',[('../escape',b'bad')]),('absolute',[('/escape',b'bad')]),('backslash',[('a\\b',b'bad')]),('duplicate',[('a',b'x'),('a',b'y')]),('case',[('A',b'x'),('a',b'y')]),('prefix',[('a',b'x'),('a/b',b'y')]),('device',[('CON',b'x')]),('bomb',[('huge',b'0'*(64*1024*1024))])]
    for name,items in attacks:
        arc=ROOT/('attacks/'+name+'.zip');arc.parent.mkdir(exist_ok=True)
        with zipfile.ZipFile(arc,'w',zipfile.ZIP_DEFLATED) as z:
            for n,b in items:z.writestr(n,b)
        sid,_,r=execute(p,'zipExtract',[arc]);check('archive-reject-'+name,r['results'][0]['state']=='failed' and not arc.with_suffix('').exists());p.dispose(sid)
    arc=ROOT/'attacks/link.zip'
    with zipfile.ZipFile(arc,'w') as z:
        e=zipfile.ZipInfo('link');e.create_system=3;e.external_attr=0o120777<<16;z.writestr(e,'../outside/untouched')
    sid,_,r=execute(p,'zipExtract',[arc]);check('archive-symlink',r['results'][0]['state']=='failed');p.dispose(sid)
    # Cancellation can land at any await. Verify every final file is either an
    # exact old snapshot or exact committed snapshot, and never replay the batch.
    files=[write('cancel/'+str(i),MP4+b'x'*(512*1024)) for i in range(80)]
    sid,v,f,_=prepare(p,'modifyHash',files);aid=action(v,'submit-form')
    rid=p.start('session.call',{'sessionId':sid,'operation':'extension:invoke-action','payload':{'actionId':aid,'formValues':f}})
    time.sleep(.02);p.send({'type':'cancel','requestId':rid,'reason':'synthetic cancellation'})
    p.wait(rid,allow_error=True);v=p.call('session.call',{'sessionId':sid,'operation':'extension:refresh-session','payload':{}})
    changed=0
    for file in files:
        data=file.read_bytes();check('cancel-atomic-file',data in [MP4+b'x'*(512*1024),MP4+b'x'*(512*1024)+MARK]);changed+=data.endswith(MARK)
    states=results(v);check('cancel-recovery-records',sum(x['state']=='committed' for x in states['results'])==changed)
    old=[x.read_bytes() for x in files];p.action(sid,aid,f);check('cancel-retry-inert',old==[x.read_bytes() for x in files]);p.dispose(sid)
    check('drop-stage-cleanup',not list(ROOT.rglob('.modify-hash-*.tmp')))
    # Canceled opens do not require dispose. Successful race winners are disposed.
    canceled=0;attempts=0
    while canceled<140 and attempts<400:
        sid=str(uuid.uuid4());rid=p.start('session.open',{'sessionId':sid,'commandName':'modifyHash','preferences':{},'argumentValues':{}})
        p.send({'type':'cancel','requestId':rid,'reason':'open dropped without dispose'});v=p.wait(rid,allow_error=True);attempts+=1
        if 'error'in v:canceled+=1
        else:p.dispose(sid)
    check('140-canceled-opens',canceled==140)
    sid,v=p.open();check('capacity-after-canceled-opens',v['root']['type']=='Form');p.dispose(sid)
    p.close()
    check('sdk-temp-unused',not (ROOT/'unused-sdk-temp').exists())
    (QA/'fixture-original-hashes.json').write_text(json.dumps(hashes,indent=2)+'\n')
    (QA/'process-results.json').write_text(json.dumps({'binary':str(BINARY),'binarySha256':hashlib.sha256(BINARY.read_bytes()).hexdigest(),'checks':counts,'cancelledOpens':canceled,'openAttempts':attempts,'cancelledBatchCommitted':changed},indent=2)+'\n')
    print('PASS production-pipe checks',sum(counts.values()),'cases',len(counts),'cancelled opens',canceled,flush=True)
if __name__=='__main__':main()
