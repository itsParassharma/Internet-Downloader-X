"""Generate small, original synthetic HLS/DASH media for offline acceptance tests."""
import pathlib,subprocess,sys
root=pathlib.Path(__file__).resolve().parents[1]
out=pathlib.Path(sys.argv[1]).resolve();out.mkdir(parents=True,exist_ok=True)
ff=root/'media-tools/ffmpeg.exe'
def run(*args):
    subprocess.run([str(ff),'-hide_banner','-loglevel','error','-nostdin','-y',*map(str,args)],check=True,cwd=out,creationflags=0x08000000 if sys.platform=='win32' else 0)
run('-f','lavfi','-i','testsrc2=size=320x180:rate=12','-f','lavfi','-i','sine=frequency=440:sample_rate=48000','-t','3','-c:v','libopenh264','-g','12','-b:v','300k','-c:a','aac','-b:a','64k','source.mp4')
for name in ['hls','dash','encrypted','split','range']:(out/name).mkdir(exist_ok=True)
run('-i','source.mp4','-c','copy','-hls_time','1','-hls_list_size','0','-hls_segment_filename','hls/seg-%03d.ts','hls/video.m3u8')
run('-i','source.mp4','-map','0:v','-map','0:a','-c','copy','-seg_duration','1','-use_template','1','-use_timeline','1','-f','dash','dash/main.mpd')
(out/'encrypted/key.bin').write_bytes(bytes(range(16)))
(out/'encrypted/key-info.txt').write_text('key.bin\n'+str(out/'encrypted/key.bin')+'\n',encoding='utf-8')
run('-i','source.mp4','-c','copy','-hls_time','1','-hls_list_size','0','-hls_key_info_file','encrypted/key-info.txt','-hls_segment_filename','encrypted/seg-%03d.ts','encrypted/video.m3u8')
for kind,selector in [('video','0:v'),('audio','0:a')]:
    run('-i','source.mp4','-map',selector,'-c','copy','-hls_time','1','-hls_list_size','0','-hls_segment_type','fmp4','-hls_fmp4_init_filename',kind+'-init.mp4','-hls_segment_filename','split/'+kind+'-%03d.m4s','split/'+kind+'.m3u8')
(out/'split/master.m3u8').write_text('#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="audio",DEFAULT=YES,NAME="Test tone",URI="audio.m3u8"\n#EXT-X-STREAM-INF:BANDWIDTH=400000,RESOLUTION=320x180,AUDIO="audio"\nvideo.m3u8\n',encoding='utf-8')
run('-i','source.mp4','-c','copy','-hls_time','1','-hls_list_size','0','-hls_flags','single_file','-hls_segment_type','fmp4','range/video.m3u8')
print('Generated clear TS, AES-128 HLS, separate fMP4 audio/video, byte ranges and DASH fixtures.')
