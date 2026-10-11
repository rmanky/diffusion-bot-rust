use fframes::{AudioMap, Color, Duration, FFramesContext, Frame, Svgr, Video, include_media_dir};

include_media_dir!(pub struct Portal2MusicVideoMedia, "media");

pub const WIDTH: usize = 1920;
pub const HEIGHT: usize = 1080;
pub const FPS: usize = 30;
pub const DURATION: f64 = 310.959;

const FONT: &str = "DejaVu Sans";
const BLUE: &str = "#39a8ff";
const ORANGE: &str = "#ff9b32";

pub struct Portal2MusicVideo<'a> { pub media: &'a Portal2MusicVideoMedia, pub title: String }
impl<'a> Portal2MusicVideo<'a> { pub fn new(media: &'a Portal2MusicVideoMedia, title: &str) -> Self { Self { media, title: title.to_owned() } } }
impl std::fmt::Debug for Portal2MusicVideo<'_> { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("Portal2MusicVideo").finish() } }

fn c(x:f64)->f64{x.clamp(0.0,1.0)}
fn p(t:f64,a:f64,b:f64)->f64{c((t-a)/(b-a))}
fn a(t:f64,s:f64,e:f64)->f64{let f=0.65;c((t-s)/f).min(c((e-t)/f))}
fn lyric(t:f64)->&'static str {
    const C:&[(f64,&str)] = &[
        (21.246,"Wake me from the endless sleep"),(26.355,"Broken halls beneath me creak"),(31.672,"Wheatley calls through shattered walls"),(36.873,"As the old facility falls"),
        (42.144,"Through the chambers, through the flame"),(47.345,"Every test still knows my name"),(52.547,"Then a voice I thought was gone"),(57.771,"Wakes beneath the ruined dawn"),
        (62.996,"Deep in Aperture I run"),(65.596,"Testing till the work is done"),(68.197,"GLaDOS watches from her throne"),(70.821,"But I will never test alone"),
        (73.421,"Through the portals, blue and gold"),(75.999,"Past the secrets left untold"),(78.646,"Every chamber, every fall"),(81.247,"I will find a way through all"),
        (83.870,""),(109.946,"Wheatley takes the central core"),(115.125,"Power changes something more"),(120.326,"Down I fall through buried years"),(125.527,"Past forgotten pioneers"),
        (130.798,"Cave Johnson through speakers cries"),(136.022,"Science lives and reason dies"),(141.224,"Painted walls and testing spheres"),(146.425,"Hold the ghosts of fifty years"),
        (151.650,"Potato circuits, weakened mind"),(156.897,"Now my enemy walks beside"),(162.122,"Old machines begin to groan"),(167.346,"As we climb toward Wheatley's throne"),
        (172.547,"Deep in Aperture I run"),(175.148,"Testing till the work is done"),(177.772,"GLaDOS watches from my side"),(180.373,"For once we share the same divide"),
        (182.950,"Through the portals, blue and gold"),(185.551,"Past the secrets left untold"),(188.198,"Every chamber, every fall"),(190.798,"I will find a way through all"),
        (193.422,""),(198.623,"Neurotoxin fills the air"),(203.709,"Turrets waiting everywhere"),(208.887,"Wheatley laughs above the room"),(214.181,"While the reactor seals our doom"),
        (219.498,"One white surface overhead"),(224.723,"One last shot before we're dead"),(229.947,"Portal opens through the sky"),(235.172,"And the moon pulls everything high"),
        (240.373,"Deep in Aperture we fight"),(242.974,"Earth below and stars in sight"),(245.597,"Wheatley vanishes through space"),(248.198,"Silence fills the testing place"),
        (250.776,"Through the portals, blue and gold"),(253.423,"Now the final door unfolds"),(256.023,"After everything we've known"),(258.647,"GLaDOS lets me walk alone"),
        (261.248,""),(287.300,"Turrets sing beneath the ground"),(292.548,"One last song, one final sound"),(297.773,"Fields of wheat beneath the sun"),(302.974,"The test is over"),(307.548,"I am done")
    ];
    let mut x=""; for &(s,l) in C { if t<s {break} x=l; } x
}

impl Video for Portal2MusicVideo<'_> {
    const FPS:usize=FPS; const WIDTH:usize=WIDTH; const HEIGHT:usize=HEIGHT; const BACKGROUND_COLOR:Color=Color::BLACK;
    fn duration(&self)->Duration<'_>{Duration::Seconds(DURATION)}
    fn audio(&self)->AudioMap<'_>{AudioMap::none()}
    fn render_frame<'a>(&'a self, frame:Frame, _ctx:&FFramesContext<'a,'_>)->Svgr<'a>{
        let t=frame.seconds(); let l=lyric(t);
        let s0=a(t,0.0,21.246); let s1=a(t,20.596,42.144); let s2=a(t,41.494,62.996); let s3=a(t,62.346,83.870);
        let s4=a(t,83.220,109.946); let s5=a(t,109.296,120.326); let s6=a(t,119.676,130.798); let s7=a(t,130.148,151.650);
        let s8=a(t,151.000,193.422); let s9=a(t,192.772,219.498); let s10=a(t,218.848,240.373); let s11=a(t,239.723,287.300); let s12=a(t,286.650,DURATION);
        let q3=p(t,62.346,83.870); let q4=p(t,83.220,109.946); let q6=p(t,119.676,130.798); let q8=p(t,151.0,193.422); let q10=p(t,218.848,240.373); let q11=p(t,239.723,287.3); let q12=p(t,286.65,DURATION);
        let scan=(t*120.0)%1080.0; let star=(t*11.0)%500.0;
        fframes::svgr!(
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1920 1080" width={WIDTH} height={HEIGHT}>
          <defs>
            <radialGradient id="bg"><stop offset="0" stop-color="#17232b"/><stop offset="1" stop-color="#030507"/></radialGradient>
            <radialGradient id="b"><stop offset="0" stop-color="#75d0ff" stop-opacity=".7"/><stop offset="1" stop-color="#39a8ff" stop-opacity="0"/></radialGradient>
            <radialGradient id="o"><stop offset="0" stop-color="#ffca70" stop-opacity=".7"/><stop offset="1" stop-color="#ff9b32" stop-opacity="0"/></radialGradient>
            <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#80c4ef"/><stop offset="1" stop-color="#e9f5fa"/></linearGradient>
          </defs>

          <g opacity={s0}><rect width="1920" height="1080" fill="url(#bg)"/><rect x="250" y="190" width="1420" height="700" rx="35" fill="#11181d" stroke="#4c5961" stroke-width="8"/><path d="M250 190h500l-90 110H250z" fill="#6e797e" opacity=".55"/><circle cx="960" cy="590" r="80" fill="#cad1d4"/><rect x="940" y="610" width="40" height="190" rx="16" fill="#252c30"/><line x1="0" y1={scan} x2="1920" y2={scan} stroke="#8bd9ff" opacity=".08"/><text x="960" y="120" text-anchor="middle" font-family={FONT} font-size="52" fill="#dce4e8" letter-spacing="8">"APERTURE // REACTIVATION"</text></g>

          <g opacity={s1}><rect width="1920" height="1080" fill="#12181c"/><path d="M0 0h1920v250H0z" fill="#dce2e5"/><path d="M0 830h1920v250H0z" fill="#bfc8cc"/><path d="M0 250h1920l-330 240H330z" fill="#69747a"/><ellipse cx="390" cy="565" rx="35" ry="185" fill="none" stroke={BLUE} stroke-width="24"/><ellipse cx="1530" cy="565" rx="35" ry="185" fill="none" stroke={ORANGE} stroke-width="24"/><circle cx="390" cy="565" r="270" fill="url(#b)" opacity=".18"/><circle cx="1530" cy="565" r="270" fill="url(#o)" opacity=".15"/><path d="M850 770l70-220 40-22 40 22 70 220z" fill="#1b2226"/></g>

          <g opacity={s2}><rect width="1920" height="1080" fill="#020304"/><path d="M960 0c-130 190-100 330-120 470-20 150-100 210-180 280" fill="none" stroke="#687177" stroke-width="50"/><path d="M960 0c130 190 100 330 120 470 20 150 100 210 180 280" fill="none" stroke="#4d555a" stroke-width="34"/><path d="M780 340c0-120 75-165 180-165s180 45 180 165l-45 190c-30 110-80 165-135 165s-105-55-135-165z" fill="#c2c9cc" stroke="#30373b" stroke-width="12"/><ellipse cx="960" cy="470" rx="130" ry="80" fill="#14191c"/><circle cx="960" cy="470" r="48" fill="#ffcf72"/><circle cx="960" cy="470" r="14" fill="#fff8dc"/><text x="960" y="900" text-anchor="middle" font-family={FONT} font-size="74" font-weight="700" fill="#eef3f5">"HELLO AGAIN"</text></g>

          <g opacity={s3}><rect width="1920" height="1080" fill="#eef2f3"/><rect y="850" width="1920" height="230" fill="#c1c9cd"/><g stroke="#9aa4a9" stroke-width="4" opacity=".45"><line x1="320" y1="0" x2="320" y2="850"/><line x1="720" y1="0" x2="720" y2="850"/><line x1="1120" y1="0" x2="1120" y2="850"/><line x1="1520" y1="0" x2="1520" y2="850"/></g><ellipse cx="230" cy="500" rx="38" ry="190" fill="none" stroke={BLUE} stroke-width="24"/><ellipse cx="1690" cy="500" rx="38" ry="190" fill="none" stroke={ORANGE} stroke-width="24"/><g transform={format!("translate({} 0)",230.0+q3*1320.0)}><circle cx="0" cy="605" r="32" fill="#20262a"/><path d="M-30 640L-65 780H60L28 640z" fill="#20262a"/><line x1="-12" y1="710" x2="-90" y2="820" stroke="#20262a" stroke-width="24"/><line x1="15" y1="710" x2="105" y2="790" stroke="#20262a" stroke-width="24"/></g><text x="960" y="120" text-anchor="middle" font-family={FONT} font-size="56" font-weight="700" fill="#182026">"TEST CHAMBERS"</text></g>

          <g opacity={s4}><rect width="1920" height="1080" fill="#030506"/><rect x="300" width="1320" height="1080" fill="#11171a"/><g stroke="#4f5a60" stroke-width="20"><line x1="500" y1="0" x2="500" y2="1080"/><line x1="1420" y1="0" x2="1420" y2="1080"/><line x1="650" y1={-300.0+q4*1500.0} x2="1270" y2={-300.0+q4*1500.0}/></g><circle cx="960" cy={120.0+q4*850.0} r="32" fill="#d9dfe2"/><text x="960" y="980" text-anchor="middle" font-family={FONT} font-size="56" fill="#cad2d6">"FALLING DEEPER"</text></g>

          <g opacity={s5}><rect width="1920" height="1080" fill="#11161a"/><circle cx="960" cy="520" r="390" fill="#232b30" stroke="#59656c" stroke-width="34"/><circle cx="960" cy="520" r="250" fill="#070a0c" stroke="#87939a" stroke-width="18"/><circle cx="960" cy="520" r="145" fill="#dbe1e3"/><circle cx="960" cy="520" r="75" fill="#252d32"/><circle cx="960" cy="520" r="35" fill="#73c6ff"/><path d="M960 0v130M960 910v170M0 520h570M1350 520h570" stroke="#a0aaaf" stroke-width="40"/><text x="960" y="980" text-anchor="middle" font-family={FONT} font-size="54" fill="#eef3f5">"CORE TRANSFER"</text></g>

          <g opacity={s6}><rect width="1920" height="1080" fill="#050606"/><circle cx="960" cy="540" r="500" fill="#17191a" stroke="#3d4346" stroke-width="18"/><circle cx="960" cy="540" r="370" fill="#070808" stroke="#292e30" stroke-width="14"/><circle cx="960" cy="540" r="250" fill="#151718" stroke="#4b5255" stroke-width="12"/><circle cx="960" cy={-100.0+q6*1250.0} r="30" fill="#e1e5e7"/><text x="120" y="150" font-family={FONT} font-size="48" fill="#d9a52b">"1950s // 1960s // 1970s"</text></g>

          <g opacity={s7}><rect width="1920" height="1080" fill="#090806"/><circle cx="450" cy="550" r="300" fill="#302713" stroke="#9b7927" stroke-width="16"/><circle cx="450" cy="550" r="210" fill="#070706" stroke="#d3a632" stroke-width="10"/><circle cx="1450" cy="550" r="300" fill="#211c10" stroke="#806624" stroke-width="14"/><circle cx="1450" cy="550" r="205" fill="#080808" stroke="#b9902c" stroke-width="9"/><rect x="800" y="160" width="320" height="760" fill="#201d16" stroke="#78683d" stroke-width="8"/><text x="960" y="110" text-anchor="middle" font-family={FONT} font-size="68" fill="#e4bf58">"OLD APERTURE"</text></g>

          <g opacity={s8}><rect width="1920" height="1080" fill="#101417"/><path d="M0 870L1920 700v380H0z" fill="#303a3f"/><path d="M0 640L1920 430" stroke="#89949a" stroke-width="18"/><ellipse cx={480.0+q8*820.0} cy={680.0-q8*190.0} rx="105" ry="72" fill="#99713c" stroke="#4d391f" stroke-width="8"/><circle cx={480.0+q8*820.0} cy={675.0-q8*190.0} r="24" fill="#ffd36b"/><ellipse cx="1510" cy="350" rx="30" ry="150" fill="none" stroke={ORANGE} stroke-width="20"/><text x="130" y="145" font-family={FONT} font-size="64" fill="#e9eef0">"UNLIKELY ALLIES"</text></g>

          <g opacity={s9}><rect width="1920" height="1080" fill="#080b0d"/><circle cx="960" cy="510" r="360" fill="#171d20" stroke="#4d585f" stroke-width="28"/><circle cx="960" cy="510" r="255" fill="#0b0f11" stroke="#e15d46" stroke-width="15"/><circle cx="960" cy="510" r={170.0+20.0*(t*4.0).sin().abs()} fill="#35130f" stroke="#ff674e" stroke-width="8"/><circle cx="340" cy="280" r="250" fill="#b8ef56" opacity=".16"/><circle cx="1580" cy="770" r="300" fill="#b8ef56" opacity=".13"/><path d="M240 760L300 590l60 170zM1560 760l60-170 60 170z" fill="#e8edef" stroke="#5d686e" stroke-width="7"/><line x1="300" y1="660" x2="960" y2="510" stroke="#ff4b42" stroke-width="5"/><line x1="1620" y1="660" x2="960" y2="510" stroke="#ff4b42" stroke-width="5"/><text x="960" y="120" text-anchor="middle" font-family={FONT} font-size="68" fill="#f0f3f4">"ONE LAST TEST"</text></g>

          <g opacity={s10}><rect width="1920" height="1080" fill="#05070a"/><circle cx="960" cy="390" r="280" fill="#e2e6e8"/><circle cx="865" cy="330" r="55" fill="#c7cdd1"/><circle cx="1050" cy="455" r="70" fill="#c4cbd0"/><ellipse cx="960" cy="520" rx={65.0+q10*75.0} ry={160.0+q10*100.0} fill="none" stroke={BLUE} stroke-width="28"/><circle cx="960" cy="520" r={260.0+q10*130.0} fill="url(#b)" opacity=".18"/><g transform={format!("rotate({} 960 650)",q10*170.0)}><rect x="400" y="720" width="120" height="65" fill="#89949a"/><rect x="1350" y="620" width="90" height="150" fill="#b7c0c4"/><circle cx="1230" cy="850" r="58" fill="#cfd5d8"/></g><text x="960" y="950" text-anchor="middle" font-family={FONT} font-size="56" fill="#edf4f7">"ONE WHITE SURFACE"</text></g>

          <g opacity={s11}><rect width="1920" height="1080" fill="#010207"/><g fill="#fff" opacity=".85"><circle cx={160.0+star} cy="140" r="3"/><circle cx={520.0+star*.3} cy="280" r="4"/><circle cx={870.0+star*.6} cy="125" r="2"/><circle cx={1680.0-star*.4} cy="175" r="4"/><circle cx={1490.0+star*.4} cy="820" r="3"/></g><circle cx="960" cy="1320" r="620" fill="#183c67" stroke="#85c8ff" stroke-width="12"/><g transform={format!("translate(0 {}) rotate({} 1040 470)",(q11-.5)*80.0,q11*25.0)}><circle cx="1040" cy="470" r="160" fill="#d4dade" stroke="#555f65" stroke-width="10"/><circle cx="1040" cy="470" r="82" fill="#222a2e"/><circle cx="1040" cy="470" r="34" fill="#79cbff"/></g><text x="960" y="900" text-anchor="middle" font-family={FONT} font-size="64" fill="#f0f6ff">"SPACE"</text></g>

          <g opacity={s12}><rect width="1920" height="1080" fill="url(#sky)"/><rect y="650" width="1920" height="430" fill="#d5b84c"/><g stroke="#8e742a" stroke-width="5" opacity=".75"><line x1="100" y1="1080" x2="120" y2="650"/><line x1="320" y1="1080" x2="300" y2="660"/><line x1="600" y1="1080" x2="630" y2="645"/><line x1="900" y1="1080" x2="880" y2="665"/><line x1="1200" y1="1080" x2="1230" y2="650"/><line x1="1500" y1="1080" x2="1470" y2="670"/><line x1="1800" y1="1080" x2="1770" y2="650"/></g><rect x="0" width={(1.0-q12)*760.0} height="1080" fill="#101417"/><rect x={1920.0-(1.0-q12)*760.0} width={(1.0-q12)*760.0} height="1080" fill="#101417"/><text x="960" y="190" text-anchor="middle" font-family={FONT} font-size="76" fill="#27323a" opacity={c((q12-.45)/.25)}>"THE TEST IS OVER"</text></g>

          <g opacity={if l.is_empty(){0.0}else{1.0}}><rect x="330" y="944" width="1260" height="94" rx="30" fill="#000" opacity=".68"/><text x="960" y="1007" text-anchor="middle" font-family={FONT} font-size="40" font-weight="700" fill="#fff">{l}</text></g>
          <rect x="28" y="28" width="1864" height="1024" rx="20" fill="none" stroke="#fff" stroke-width="2" opacity=".08"/>
        </svg>)
    }
}
