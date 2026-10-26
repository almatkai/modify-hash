from process_qa import *
def main():
    p=Pipe();oracle=json.loads((QA/'oracle-results.json').read_text());comparisons=[]
    for e in oracle['evidence']:
        before=base64.b64decode(e['base64']);f=write('parity/'+e['name'],before)
        sid,_,r=execute(p,'modifyHash',[f],{'enableImage':True,'enableAudio':True})
        expected=e['category'] in ['image','audio','video'];actual=f.read_bytes()==before+MARK

        if actual!=expected:print('PARITY MISMATCH',e['name'],e['type'],r,flush=True)
        check('locked-file-type-parity',actual==expected)
        comparisons.append({'fixture':e['name'],'expected':e['category'],'state':r['results'][0]['state'],'matched':actual==expected});p.dispose(sid)
    # Recursive directory processing, overlapping selections, and no-follow in trees.
    a=write('recursive/sub/a',MP4);b=write('recursive/b',MP4);symlink=ROOT/'recursive/sub/link';symlink.symlink_to(ROOT/'parity/gif')
    sid,_,r=execute(p,'modifyHash',[ROOT/'recursive',a]);check('recursive-and-overlap',a.read_bytes()==MP4+MARK and b.read_bytes()==MP4+MARK and len([x for x in r['results']if x['state']=='committed'])==2);p.dispose(sid)
    # Valid source files with literal punctuation/newlines are not split or narrowed.
    a=write('ordinary/new\nline " é',MP4);sid,_,r=execute(p,'modifyHash',[a]);check('ordinary-punctuation',a.read_bytes()==MP4+MARK);p.dispose(sid)
    a=write('readonly-dest/a',MP4);a.parent.chmod(0o555)
    try:
        sid,_,r=execute(p,'modifyHash',[a]);check('destination-readonly',a.read_bytes()==MP4 and r['results'][0]['state']=='failed');p.dispose(sid)
    finally:a.parent.chmod(0o755)
    # CRC failures and cross-archive conflicts never publish partially extracted trees.
    arc=ROOT/'crc.zip'
    with zipfile.ZipFile(arc,'w',zipfile.ZIP_STORED)as z:z.writestr('item',b'original')
    data=bytearray(arc.read_bytes());at=data.index(b'original');data[at]^=1;arc.write_bytes(data)
    sid,_,r=execute(p,'zipExtract',[arc]);check('crc-rejection',r['results'][0]['state']=='failed' and not ROOT.joinpath('crc').exists());p.dispose(sid)
    arcs=[]
    for n in ['cross1.zip','cross2.zip']:
        arc=ROOT/n
        with zipfile.ZipFile(arc,'w')as z:z.writestr('same',n.encode())
        arcs.append(arc)
    sid,_,r=execute(p,'zipExtract',arcs);check('cross-archive-conflict',r['results'][0]['state']=='failed');p.dispose(sid)
    # ZIP64 metadata is supported without materializing a giant file.
    arc=ROOT/'zip64.zip'
    with zipfile.ZipFile(arc,'w',allowZip64=True)as z:
        with z.open('item','w',force_zip64=True)as out:out.write(b'zip64')
    sid,_,r=execute(p,'zipExtract',[arc]);check('zip64-local-header',r['results'][0]['state']=='committed' and pathlib.Path(r['results'][0]['path']).joinpath('item').read_bytes()==b'zip64');p.dispose(sid)
    # Explicit no-password rejection is effect free for both archive commands.
    for cmd in ['zipCompress','zipExtract']:
        sid,v,f,field=prepare(p,cmd,[arc],{'enableZipPassword':True});check('password-form-error',not any(a['kind']=='submit-form' for a in v['actions']));p.dispose(sid)
    # Concurrent action IDs never queue a second effect in this or another session.
    big=write('concurrent/a',MP4+b'x'*(64*1024*1024));other=write('concurrent/b',MP4)
    sid,v,f,_=prepare(p,'modifyHash',[big]);aid=action(v,'submit-form');sid2,v2,f2,_=prepare(p,'modifyHash',[other]);aid2=action(v2,'submit-form')
    rid=p.start('session.call',{'sessionId':sid,'operation':'extension:invoke-action','payload':{'actionId':aid,'formValues':f}})
    duplicate=p.start('session.call',{'sessionId':sid,'operation':'extension:invoke-action','payload':{'actionId':aid,'formValues':f}})
    v2=p.action(sid2,aid2,f2,allow_error=True);p.wait(duplicate,allow_error=True);p.wait(rid,timeout=30)
    check('concurrent-same-token',big.stat().st_size==64*1024*1024+len(MP4)+len(MARK))
    check('concurrent-other-session',other.read_bytes()==MP4 and 'Another filesystem batch'in results(v2)['status'])
    p.dispose(sid);p.dispose(sid2);p.close()
    # EOF drops an active staged replacement; product has no detached workers.
    p=Pipe();f=write('eof/a',MP4+b'x'*(64*1024*1024));sid,v,values,_=prepare(p,'modifyHash',[f]);rid=p.start('session.call',{'sessionId':sid,'operation':'extension:invoke-action','payload':{'actionId':action(v,'submit-form'),'formValues':values}})
    time.sleep(.01);p.p.stdin.close();p.p.wait(timeout=5);p.reader.join(1);p.stderr.join(1)
    check('eof-stage-cleanup',not list(ROOT.rglob('.modify-hash-*.tmp')) and f.stat().st_size in [64*1024*1024+len(MP4),64*1024*1024+len(MP4)+len(MARK)])
    # Oversized frames are rejected by the actual production SDK reader.
    p=Pipe();p.p.stdin.write(b'x'*(16*1024*1024+1)+b'\n');p.p.stdin.flush();p.p.stdin.close();p.p.wait(timeout=5);check('frame-cap',p.p.returncode!=0)
    (QA/'extended-fixture-original-hashes.json').write_text(json.dumps(hashes,indent=2)+'\n')
    (QA/'extended-results.json').write_text(json.dumps({'checks':counts,'parity':comparisons},indent=2)+'\n')
    print('PASS extended production checks',sum(counts.values()),'including',len(comparisons),'locked-library fixtures')
if __name__=='__main__':main()
