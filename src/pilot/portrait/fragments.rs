use super::{Accessory, EyeStyle, FaceShape, HairStyle, MouthStyle, PortraitDescriptor, ShirtStyle};

// ---------------------------------------------------------------------------
// Color placeholder tokens — replaced with hex values during assembly
// ---------------------------------------------------------------------------

pub const SKIN_TONE: &str = "SKIN_TONE";
pub const SKIN_SHADOW: &str = "SKIN_SHADOW";
pub const SKIN_HIGHLIGHT: &str = "SKIN_HIGHLIGHT";
pub const HAIR_COLOR: &str = "HAIR_COLOR";
pub const HAIR_SHADOW: &str = "HAIR_SHADOW";
pub const EYE_COLOR: &str = "EYE_COLOR";
pub const BG_COLOR: &str = "BG_COLOR";
pub const ACC_COLOR: &str = "ACC_COLOR";
pub const ACC_SHADOW: &str = "ACC_SHADOW";
pub const SHIRT_COLOR: &str = "SHIRT_COLOR";

// ===========================================================================
// Face shapes (4 hand-drawn + 2 fallback aliases)
// ===========================================================================
// Coordinates are in the Inkscape mm canvas (~14-24 x, ~13-31 y).
// Layer transforms stripped; portrait-03 fragments wrapped in translate(-28,0).

const FACE_OVAL: &str = r#"<g id="face">
  <path style="fill:SKIN_TONE;fill-opacity:1;stroke:none;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 22.774891,28.740321 c -0.568485,-1.395373 -2.524526,-1.441619 -1.316941,-3.916578 0.411732,-0.230299 1.06176,-0.893466 1.369182,-1.295535 0.682558,-0.892696 0.772175,-1.616028 0.969379,-2.600754 1.236062,0.683857 2.057564,-3.986066 0.132809,-2.77089 0.0044,-1.08268 -0.220344,-2.119327 -0.540019,-2.856674 -0.494618,-1.140864 -2.70741,-1.900659 -4.088122,-1.900659 -1.380712,0 -3.265642,0.670615 -4.029729,1.697054 -0.490783,0.659295 -0.553233,1.717708 -0.518831,2.760039 -1.596912,-0.746676 -0.929856,3.169805 0.173633,2.938321 0.178768,1.038426 0.642606,2.294418 1.356567,3.008379 0.226206,0.226206 0.679325,0.613697 0.872586,0.769798 1.001498,1.985035 -0.890574,2.809582 -1.459059,4.204955"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 23.591328,19.314655 c 0.127305,0.362575 0.163862,0.552218 0.01169,0.818513"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 15.380992,19.028348 c -0.08411,0.268252 0.0326,0.835156 0.100981,0.954817"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 19.743295,20.225293 c 0.06527,0.433996 -0.38036,1.015791 -0.588578,1.156103 -0.224673,0.151402 0.118633,0.665966 0.637458,0.599071"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 18.500367,24.872174 c 0.573694,0.53627 1.273107,0.487325 1.87662,0.01512"/>
</g>"#;

const FACE_ROUND: &str = r#"<g id="face">
  <path style="fill:SKIN_TONE;fill-opacity:1;stroke:none;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 22.774891,29.033794 c -0.568485,-1.395373 -2.499721,-1.921176 -1.432696,-4.379599 0.116655,-0.072 0.303047,-0.184695 0.415611,-0.274551 0.225129,-0.179711 0.445658,-0.384346 0.671864,-0.610552 0.68335,-0.68335 1.169637,-1.563439 1.366841,-2.548165 1.236062,0.683857 2.057564,-3.986066 0.132809,-2.77089 0.0044,-1.08268 -0.220344,-2.119327 -0.540019,-2.856674 -0.494618,-1.140864 -2.70741,-1.900659 -4.088122,-1.900659 -1.380712,0 -3.265642,0.670615 -4.029729,1.697054 -0.490783,0.659295 -0.553233,1.717708 -0.518831,2.760039 -1.596912,-0.746676 -0.929856,3.169805 0.173633,2.938321 0.178768,1.038426 0.677685,1.967013 1.391646,2.680974 0.226206,0.226206 0.528491,0.478081 0.721752,0.634182 1.042839,2.439788 -0.774819,3.272603 -1.343304,4.667976"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 17.726651,24.648932 c 0.806282,0.511465 1.794238,0.751908 2.934954,0.02339"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 14.686461,19.235054 c -0.08411,0.268252 0.107016,0.76901 0.175395,0.888671"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 24.227982,19.422142 c 0.127305,0.362575 0.163862,0.552218 0.01169,0.818513"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 19.433835,20.357585 c -0.349018,0.694359 -0.765527,1.48382 0.42095,1.052374"/>
</g>"#;

// Portrait 03 — offset by translate(-28,0) to normalize to shared coord space
const FACE_SQUARE: &str = r#"<g id="face" transform="translate(-28,0)">
  <path style="fill:SKIN_TONE;fill-opacity:1;stroke:none;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 50.774445,29.181282 c -0.568485,-1.395373 -2.476335,-1.348217 -1.40931,-3.80664 0.601316,-0.272216 1.065052,-0.532243 1.449959,-0.908489 0.68335,-0.68335 0.783767,-2.113012 0.980971,-3.097738 1.318744,1.270901 2.413098,-3.96953 0.132809,-2.77089 0.0044,-1.08268 -0.220344,-2.119327 -0.540019,-2.856674 -0.494618,-1.140864 -2.70741,-1.900659 -4.088122,-1.900659 -1.380712,0 -3.265642,0.670615 -4.029729,1.697054 -0.490783,0.659295 -0.553233,1.717708 -0.518831,2.760039 -1.712667,-0.878968 -1.392877,3.450925 0.173633,2.938321 0.178768,1.038426 0.432131,2.434735 1.146092,3.148696 0.226206,0.226206 0.797431,0.583318 0.990692,0.739419 1.042839,2.439788 -0.798205,2.699644 -1.36669,4.095017"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 45.726205,24.79642 c 0.806282,0.511465 1.794238,0.751908 2.934954,0.02339"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 47.416853,20.149539 c -0.90299,1.347549 -0.517481,1.897232 0.528437,1.614614"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 52.244072,19.263706 c 0.127305,0.362575 0.0233,1.246749 -0.128869,1.513044"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 42.652942,18.894716 c -0.125451,0.293057 0.09875,1.579297 0.167127,1.698958"/>
</g>"#;

const FACE_ANGULAR: &str = r#"<g id="face">
  <path style="fill:SKIN_TONE;fill-opacity:1;stroke:none;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 22.774891,29.033794 c -0.568485,-1.395373 -2.184009,-1.874404 -1.432696,-4.379599 0.215614,-1.18294 0.85133,-1.072999 1.435916,-1.52435 0.570097,-0.440165 0.79781,-0.87742 1.0184,-1.908918 1.236062,0.683857 2.057564,-3.986066 0.132809,-2.77089 0.0044,-1.08268 -0.220344,-2.119327 -0.540019,-2.856674 -0.494618,-1.140864 -2.70741,-1.900659 -4.088122,-1.900659 -1.380712,0 -3.265642,0.670615 -4.029729,1.697054 -0.490783,0.659295 -0.553233,1.717708 -0.518831,2.760039 -1.596912,-0.746676 -0.929856,3.169805 0.173633,2.938321 0.178768,1.038426 0.326135,1.427404 0.774751,1.682668 0.719343,0.409308 1.206932,1.000985 1.338647,1.632488 0.968425,2.605153 -0.774819,3.272603 -1.343304,4.667976"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 17.731071,24.279719 c 1.387731,1.223201 1.502457,1.482065 3.114055,-0.176795"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 14.686461,19.235054 c -0.08411,0.268252 0.107016,0.76901 0.175395,0.888671"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 24.227982,19.422142 c 0.127305,0.362575 0.163862,0.552218 0.01169,0.818513"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 19.416295,20.497902 c -0.313938,0.267562 -0.747987,0.764697 0.43849,0.912057"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" d="m 22.626297,21.104627 c -0.485067,0.154951 -0.848398,0.412538 -0.9778,0.555705"/>
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" d="m 16.382153,21.393965 c 0.485067,0.154951 0.848398,0.412538 0.9778,0.555705"/>
</g>"#;

// ===========================================================================
// Eyes (4 hand-drawn + 2 fallback aliases)
// ===========================================================================

const EYES_NORMAL: &str = r#"<g id="eyes">
  <g transform="translate(-0.1267629)">
    <g transform="matrix(1.5075324,0,0,1.5075324,-9.0076282,-10.273431)">
      <path style="fill:SKIN_HIGHLIGHT;fill-opacity:1;stroke:none;stroke-width:0.25" d="m 18.501841,19.693152 c -0.255982,-0.503819 -1.517751,-1.01813 -1.721592,-0.0124 0.553871,0.720476 1.447705,0.611309 1.721592,0.0124 z"/>
      <path style="fill:EYE_COLOR;fill-opacity:1;stroke:none;stroke-width:0.25" d="m 17.460432,19.108395 a 0.60000002,0.60000002 0 0 0 -0.369486,0.553971 0.60000002,0.60000002 0 0 0 0.162264,0.410311 c 0.284595,0.141413 0.587768,0.145811 0.834057,0.03979 a 0.60000002,0.60000002 0 0 0 0.203606,-0.450102 0.60000002,0.60000002 0 0 0 -0.07183,-0.284737 c -0.214215,-0.154693 -0.498391,-0.267785 -0.75861,-0.269234 z"/>
      <circle style="fill:#000000;stroke:none;stroke-width:0.25" cx="17.772558" cy="19.711458" r="0.25"/>
    </g>
    <g transform="matrix(-1.5075324,0,0,1.5075324,48.138508,-10.273431)">
      <path style="fill:SKIN_HIGHLIGHT;fill-opacity:1;stroke:none;stroke-width:0.25" d="m 18.501841,19.693152 c -0.255982,-0.503819 -1.517751,-1.01813 -1.721592,-0.0124 0.553871,0.720476 1.447705,0.611309 1.721592,0.0124 z"/>
      <path style="fill:EYE_COLOR;fill-opacity:1;stroke:none;stroke-width:0.25" d="m 17.460432,19.108395 a 0.60000002,0.60000002 0 0 0 -0.369486,0.553971 0.60000002,0.60000002 0 0 0 0.162264,0.410311 c 0.284595,0.141413 0.587768,0.145811 0.834057,0.03979 a 0.60000002,0.60000002 0 0 0 0.203606,-0.450102 0.60000002,0.60000002 0 0 0 -0.07183,-0.284737 c -0.214215,-0.154693 -0.498391,-0.267785 -0.75861,-0.269234 z"/>
      <circle style="fill:#000000;stroke:none;stroke-width:0.25" cx="17.772558" cy="19.711458" r="0.25"/>
    </g>
    <g transform="translate(0.0432104)">
      <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 20.246502,18.638709 c 0.454352,-0.594608 1.153712,-0.636215 1.74811,-0.555418 0.331002,0.04499 0.706741,0.224613 0.97637,0.309866"/>
      <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 18.797957,18.753488 c -0.454352,-0.594608 -1.153712,-0.636215 -1.74811,-0.555418 -0.331002,0.04499 -0.706741,0.224613 -0.97637,0.309866"/>
    </g>
  </g>
</g>"#;

const EYES_NARROW: &str = r#"<g id="eyes">
  <g transform="translate(0.05029375,0.01019948)">
    <g transform="translate(0.27478648,-0.03507913)">
      <g transform="matrix(-1.5075324,0,0,1.5075324,48.138508,-10.273431)">
        <path style="fill:SKIN_HIGHLIGHT;fill-opacity:1;stroke:none;stroke-width:0.25" d="m 18.501841,19.693152 c -0.255982,-0.503819 -1.440187,-0.828098 -1.721592,-0.0124 0.51315,0.619643 1.447705,0.611309 1.721592,0.0124 z"/>
      </g>
      <g transform="matrix(-1.5075324,0,0,1.5075324,48.13306,-10.275322)">
        <path style="fill:EYE_COLOR;fill-opacity:1;stroke:none;stroke-width:0.25" d="m 17.539194,19.184662 c 0.246556,0.0048 0.502382,0.09189 0.695174,0.223498 a 0.60000002,0.60000002 0 0 1 0.05656,0.254348 0.60000002,0.60000002 0 0 1 -0.146028,0.391807 c -0.268585,0.129603 -0.621096,0.119487 -0.936154,-0.03462 a 0.60000002,0.60000002 0 0 1 -0.117919,-0.357186 0.60000002,0.60000002 0 0 1 0.207386,-0.453851 c 0.0774,-0.01814 0.158795,-0.02559 0.240981,-0.02399 z"/>
        <circle style="fill:#000000;stroke:none;stroke-width:0.25" cx="17.772558" cy="19.711458" r="0.25"/>
      </g>
    </g>
    <g transform="matrix(-1,0,0,1,38.802622,-0.02338609)">
      <g transform="matrix(-1.5075324,0,0,1.5075324,48.138508,-10.273431)">
        <path style="fill:SKIN_HIGHLIGHT;fill-opacity:1;stroke:none;stroke-width:0.25" d="m 18.501841,19.693152 c -0.255982,-0.503819 -1.440187,-0.828098 -1.721592,-0.0124 0.51315,0.619643 1.447705,0.611309 1.721592,0.0124 z"/>
      </g>
      <g transform="matrix(-1.5075324,0,0,1.5075324,48.13306,-10.275322)">
        <path style="fill:EYE_COLOR;fill-opacity:1;stroke:none;stroke-width:0.25" d="m 17.539194,19.184662 c 0.246556,0.0048 0.502382,0.09189 0.695174,0.223498 a 0.60000002,0.60000002 0 0 1 0.05656,0.254348 0.60000002,0.60000002 0 0 1 -0.146028,0.391807 c -0.268585,0.129603 -0.621096,0.119487 -0.936154,-0.03462 a 0.60000002,0.60000002 0 0 1 -0.117919,-0.357186 0.60000002,0.60000002 0 0 1 0.207386,-0.453851 c 0.0774,-0.01814 0.158795,-0.02559 0.240981,-0.02399 z"/>
        <circle style="fill:#000000;stroke:none;stroke-width:0.25" cx="17.772558" cy="19.711458" r="0.25"/>
      </g>
    </g>
    <g>
      <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 18.818024,18.454096 c -0.454352,-0.594608 -0.837898,-0.61748 -1.549673,-0.613296 -0.25428,0.0015 -0.987029,0.1036 -1.268351,0.239121"/>
      <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 20.205576,18.486701 c 0.198037,-0.569803 0.862703,-0.66709 1.5166,-0.629832 0.253873,0.01447 0.912615,0.186281 1.301424,0.255657"/>
    </g>
  </g>
</g>"#;

// Portrait 03 eyes — wrapped in translate(-28,0)
const EYES_WIDE: &str = r#"<g id="eyes" transform="translate(-28,0)">
  <g transform="translate(27.986676,-0.05901392)">
    <g transform="matrix(-1.5075324,0,0,1.5075324,48.138508,-10.249156)">
      <path style="fill:SKIN_HIGHLIGHT;fill-opacity:1;stroke:none;stroke-width:0.25" d="m 18.501841,19.693152 c -0.04482,-0.701265 -1.446451,-1.177184 -1.721592,-0.0124 0.241248,0.619011 1.612243,0.792301 1.721592,0.0124 z"/>
      <path style="fill:EYE_COLOR;fill-opacity:1;stroke:none;stroke-width:0.25" d="m 17.549461,18.976241 c 0.178977,-0.0062 0.361629,0.04181 0.518638,0.127175 a 0.66333568,0.66333568 0 0 1 0.214586,0.488473 0.66333568,0.66333568 0 0 1 -0.423001,0.61839 c -0.236895,0.01882 -0.497871,-0.03598 -0.707858,-0.148085 a 0.66333568,0.66333568 0 0 1 -0.195732,-0.470305 0.66333568,0.66333568 0 0 1 0.28143,-0.542291 c 0.09828,-0.04638 0.20455,-0.06964 0.311937,-0.07336 z"/>
      <circle style="fill:#000000;stroke:none;stroke-width:0.25" cx="17.772558" cy="19.711458" r="0.29850104"/>
    </g>
    <g transform="matrix(1.5075324,0,0,1.5075324,-9.1145643,-10.249156)">
      <path style="fill:SKIN_HIGHLIGHT;fill-opacity:1;stroke:none;stroke-width:0.25" d="m 18.501841,19.693152 c -0.04482,-0.701265 -1.446451,-1.177184 -1.721592,-0.0124 0.241248,0.619011 1.612243,0.792301 1.721592,0.0124 z"/>
      <path style="fill:EYE_COLOR;fill-opacity:1;stroke:none;stroke-width:0.25" d="m 17.549461,18.976241 c 0.178977,-0.0062 0.361629,0.04181 0.518638,0.127175 a 0.66333568,0.66333568 0 0 1 0.214586,0.488473 0.66333568,0.66333568 0 0 1 -0.423001,0.61839 c -0.236895,0.01882 -0.497871,-0.03598 -0.707858,-0.148085 a 0.66333568,0.66333568 0 0 1 -0.195732,-0.470305 0.66333568,0.66333568 0 0 1 0.28143,-0.542291 c 0.09828,-0.04638 0.20455,-0.06964 0.311937,-0.07336 z"/>
      <circle style="fill:#000000;stroke:none;stroke-width:0.25" cx="17.772558" cy="19.711458" r="0.29850104"/>
    </g>
    <g transform="translate(0.0432104)">
      <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 20.246502,18.638709 c 0.454352,-0.594608 1.153712,-0.636215 1.74811,-0.555418 0.331002,0.04499 0.706741,0.224613 0.97637,0.309866"/>
      <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 18.797957,18.753488 c -0.454352,-0.594608 -1.153712,-0.636215 -1.74811,-0.555418 -0.331002,0.04499 -0.706741,0.224613 -0.97637,0.309866"/>
    </g>
  </g>
</g>"#;

// Portrait 04 eyes — visor style (uses EYE_COLOR + ACC_COLOR for visor band)
const EYES_VISOR: &str = r#"<g id="eyes">
  <path style="fill:EYE_COLOR;fill-opacity:1;stroke:ACC_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" d="m 23.883601,18.944106 c -0.333937,0.555245 -1.674542,1.420589 -2.366632,1.460123 -0.748274,0.04274 -1.595644,-0.333381 -2.064214,-0.327302 -0.609839,0.0079 -1.291627,0.468298 -1.992747,0.342694 -0.701121,-0.125606 -2.159062,-0.803655 -2.662729,-1.718475 l 0.03326,-1.085491 c 1.089847,0.380199 2.283731,0.04843 3.002147,0.196575 0.718416,0.148146 1.230422,0.718707 1.722055,0.737462 0.492465,0.01879 0.875667,-0.668296 1.712315,-0.743537 0.772417,-0.06946 1.985712,0.09981 2.563669,-0.06031 z"/>
</g>"#;

// ===========================================================================
// Mouths (4 hand-drawn + 1 fallback alias)
// ===========================================================================

const MOUTH_NEUTRAL: &str = r#"<g id="mouth">
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 18.729405,23.741416 c 0.601921,0.251291 1.164067,0.08658 1.418543,-0.03473"/>
</g>"#;

const MOUTH_SMILE: &str = r#"<g id="mouth">
  <path style="fill:SKIN_HIGHLIGHT;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 20.680165,22.904851 -2.648934,0.05219 c 0.404971,0.635267 1.609415,0.740497 2.39417,0.170532 0.09077,-0.06593 0.177154,-0.138431 0.254764,-0.222725 z"/>
</g>"#;

// Portrait 03 mouth — wrapped in translate(-28,0)
const MOUTH_SMIRK: &str = r#"<g id="mouth" transform="translate(-28,0)">
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 45.699479,22.81302 c 0.276347,0.266936 0.785055,0.822348 1.604889,0.825342"/>
</g>"#;

const MOUTH_FROWN: &str = r#"<g id="mouth">
  <path style="fill:none;stroke:SKIN_SHADOW;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 18.651378,23.574007 c 0.383336,-0.514449 1.4344,-0.590492 1.828618,-0.324903"/>
</g>"#;

// ===========================================================================
// Hair — top (front) layer (4 hand-drawn + 3 fallback aliases)
// ===========================================================================

const HAIR_SHORT_CROP_FRONT: &str = r#"<g id="hair">
  <path style="fill:HAIR_COLOR;fill-opacity:1;stroke:none;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 21.944237,16.71038 c -0.662002,0.624491 -0.476681,0.22622 -0.87017,0.641194 -0.473999,-0.413274 -0.381961,-0.234857 -0.854121,-0.782929 -0.264354,0.214658 -0.598052,0.635291 -0.944341,0.797359 -0.07972,-0.175376 -0.08535,-0.538329 -0.18359,-0.618287 -0.09824,-0.07996 -0.309566,0.40371 -0.420642,0.354708 -0.111076,-0.049 -0.116061,-0.437738 -0.234274,-0.520241 -0.118213,-0.0825 -0.323345,0.304925 -0.422536,0.443092 -0.198371,-0.244236 -0.403437,-0.502314 -0.515534,-0.546421 -0.0687,-0.02703 -0.173435,0.682105 -0.288616,0.650725 -0.11518,-0.03139 -0.156341,-0.480774 -0.256676,-0.597361 -0.03129,0.04183 -0.212932,0.503236 -0.275966,0.548399 -0.06303,0.04516 -0.463481,-0.358074 -0.5465,-0.332502 -0.08302,0.02557 -0.322178,0.652426 -0.413423,0.635484 -0.09125,-0.01694 -0.364904,-0.353198 -0.452614,-0.435579 -0.257934,0.091 -0.181465,0.965051 -0.528618,0.936974 -0.266973,-2.044598 0.62258,-3.821379 2.517529,-4.485631 0.912011,-0.319067 2.813667,-0.326444 3.73271,-0.0877 1.275127,0.331249 1.799167,0.80007 2.360974,1.402345 0.685485,0.734862 0.59377,3.345079 0.59377,3.345079 0,0 -0.166734,-0.41086 -0.32111,-0.480893 -0.154372,-0.07003 -0.378597,-0.794005 -0.608395,-0.912786 -0.153792,0.325575 -0.211603,0.493039 -0.497138,0.676128 -0.226274,-0.146257 -0.301216,-0.531318 -0.570719,-0.631154 z"/>
</g>"#;

const HAIR_MOHAWK_FRONT: &str = r#"<g id="hair">
  <path style="fill:HAIR_COLOR;fill-opacity:1;stroke:HAIR_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" d="m 18.430687,12.879106 c 0.05908,0.05064 0.370761,0.426558 0.475425,0.558105 0.01747,-0.202627 0.118163,-0.436267 0.194334,-0.640789 0.075,-0.229699 0.108725,-0.323542 0.268719,-0.02067 0.135925,0.257305 0.178346,0.273066 0.289387,0.578776 0.04325,-0.124958 0.08715,-0.489743 0.425813,-0.760674 0.05841,-0.04673 0.282429,0.44628 0.332256,0.588252 0.30521,1.089635 0.132803,3.311851 0.132803,3.311851 0,0 -0.615085,0.316925 -0.991441,0.584454 -0.338317,-0.241061 -0.768516,-0.457969 -1.07443,-0.568886 0.222775,-1.575695 -0.349128,-3.884361 -0.05287,-3.630419 z"/>
</g>"#;

// Portrait 03 hair top — wrapped in translate(-28,0)
const HAIR_LONG_SWEPT_FRONT: &str = r#"<g id="hair" transform="translate(-28,0)">
  <path style="fill:HAIR_COLOR;fill-opacity:1;stroke:none;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 47.641144,16.046196 c -0.723032,1.014281 -0.152872,2.502503 -4.925228,2.252546 -0.176023,-1.978452 0.525905,-3.621179 2.420854,-4.285431 0.912011,-0.319067 2.813667,-0.326444 3.73271,-0.0877 1.275127,0.331249 1.799167,0.80007 2.360974,1.402345 0.685485,0.734862 0.722393,3.251535 0.722393,3.251535 -4.25091,-0.01655 -3.635834,-1.672886 -4.311703,-2.533295 z"/>
</g>"#;

const HAIR_BEANIE_FRONT: &str = r#"<g id="hair">
  <path style="fill:HAIR_COLOR;fill-opacity:1;stroke:HAIR_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" d="m 14.821604,17.6044 c -0.340964,-5.230794 8.90828,-5.606837 9.004102,0.14056 -3.730155,0.824342 -4.675513,0.798491 -9.004102,-0.14056 z"/>
</g>"#;

// ===========================================================================
// Hair — back layer (3 hand-drawn, rest empty or fallback)
// ===========================================================================

const HAIR_SHORT_CROP_BACK: &str = r#"<g id="hair-back">
  <path style="fill:HAIR_COLOR;fill-opacity:1;stroke:HAIR_COLOR;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 14.842548,17.956495 c 0.01987,-0.518113 -0.276983,-1.340229 -0.276983,-1.860351 0,-0.118401 -0.07924,-1.307413 -0.04134,-1.364258 0.0068,-0.01022 0.239778,0.194565 0.239778,0.198438 0,0.01654 -0.0036,-0.03347 0,-0.04961 0.02117,-0.09528 0.06998,-0.228953 0.107487,-0.314192 0.09737,-0.221308 0.240755,-0.440168 0.355534,-0.669727 0.03869,-0.07737 -0.0064,0.214829 0.132292,0.115755 0.09984,-0.07131 0.180686,-0.150707 0.281119,-0.223242 0.329687,-0.238106 0.566463,-0.517382 0.909506,-0.768945 0.187467,-0.137477 0.0779,0.262243 0.181901,0.223242 0.426156,-0.159809 0.908409,-0.218615 1.322916,-0.405143 0.0149,-0.0067 0.104257,-0.08843 0.132292,-0.07441 0.03827,0.01913 0.03361,0.174173 0.05788,0.198437 0.05758,0.05758 0.747863,-0.005 0.967382,-0.01654 0.164325,-0.0086 0.549471,-0.217225 0.686263,-0.148828 0.169704,0.08485 0.04284,0.125726 0.297657,0.165364 0.279937,0.04355 1.37455,-0.135632 1.637109,0.07441 0.03216,0.02573 -0.05625,0.297591 -0.03307,0.355534 0.0173,0.04329 0.936093,-0.11828 0.983919,-0.09095 0.201451,0.115114 0.30828,0.490181 0.479558,0.661458 0.04175,0.04175 0.131122,0.153951 0.190169,0.173633 0.09211,0.0307 0.185923,-0.246149 0.239779,-0.165365 0.15481,0.232215 0.355115,1.471326 0.405143,1.521354 0.02897,0.02897 0.243467,-0.260895 0.264583,-0.239778 0.0013,0.0012 -0.09979,0.85389 -0.165364,1.11621 -0.02601,0.103995 -0.19398,0.877728 -0.181902,0.926042 0.0085,0.03409 0.220909,-0.19377 0.214974,-0.181901 -0.13648,0.272958 -0.19114,0.599979 -0.347265,0.818555"/>
</g>"#;

// Portrait 03 hair back — wrapped in translate(-28,0)
const HAIR_LONG_SWEPT_BACK: &str = r#"<g id="hair-back" transform="translate(-28,0)">
  <path style="fill:HAIR_COLOR;fill-opacity:1;stroke:none;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" d="m 39.852864,29.352213 c -0.428889,0.0547 -1.00734,-0.281123 -1.273307,-0.628389 0.238697,-1.455208 2.025346,-1.848228 2.199349,-3.45612 0.461892,-4.268131 -0.897869,-6.254815 2.199349,-10.31875 1.522338,-1.997496 3.554312,-1.959439 5.357813,-2.017447 2.38681,-0.07677 3.872393,0.837339 4.944401,2.513541 1.30347,2.038119 0.409021,5.733871 0.694532,8.516276 0.469144,4.57197 0.897513,3.898063 2.116666,5.390885 -0.48691,0.48691 -1.893888,0.470826 -2.199349,0.165365"/>
</g>"#;

const HAIR_BEANIE_BACK: &str = r#"<g id="hair-back">
  <path style="fill:HAIR_COLOR;fill-opacity:1;stroke:HAIR_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" d="m 18.137164,12.676536 c 0.09323,0.03798 0.162161,0.111835 0.239779,0.165364 m 1.050065,-0.686263 c -0.03359,0.110348 -0.06722,0.235255 -0.09922,0.347266 m -0.429947,-0.347266 c -0.02669,0.138199 0.02233,0.273545 0.03307,0.405144 m -0.297653,-0.388607 c -0.009,0.143231 -0.01654,0.286862 -0.02479,0.429948 M 18.335602,12.46983 c 0.06679,0.146311 0.174466,0.245343 0.264583,0.37207 m -0.463021,0.05788 c 0.03134,0.210093 0.265197,0.235731 0.37207,0.330729 m 0.446485,-0.909505 c -0.02795,0.12953 0.09409,0.282268 0.124023,0.37207 m 1.604037,0.669727 c -0.128236,0.0571 -0.271849,0.03883 -0.405144,0.05788 m 0.181902,-0.471292 c -0.04671,0.0654 -0.11025,0.115762 -0.165365,0.173633 m -0.545703,-0.818555 c -0.09942,0.08765 -0.09574,0.214005 -0.14056,0.314193 m -0.03307,-0.388607 c -0.02956,0.10203 -0.03879,0.210562 -0.05788,0.314193 m 0.603583,0.01654 c -0.02188,0.137552 -0.137059,0.234143 -0.198437,0.338997 M 18.3108,13.032075 c 0.07304,0.07475 0.165428,0.121314 0.248047,0.181901 m -0.520899,0.223242 c 0.09922,0 0.198438,0 0.297657,0 m 0.07441,-0.686263 c 7.93e-4,-0.01666 0.119107,0.06732 0.190169,0.107487 m 0.07441,-0.438216 c 0.01479,0.111382 0.109082,0.212103 0.148828,0.289388 m 0.454753,-0.496094 c 0,0.104731 0,0.209462 0,0.314193 m 0.421679,0.0248 c 0.06267,-0.09024 0.122881,-0.178735 0.181901,-0.264583 m 0.231511,0.744141 c 0.06811,-0.108939 0.192926,-0.110244 0.289388,-0.165365 m -0.28112,0.677995 c 0.148873,-0.0097 0.297617,-0.01653 0.446485,-0.02481 m 0.148828,-0.471284 c -0.168244,0.106304 -0.3785,0.16631 -0.545703,0.239779 m 0.173632,-0.611849 c -0.147362,0.141055 -0.314333,0.259187 -0.471289,0.388607 m -1.5875,-0.239779 c 0.0498,-0.02046 0.267108,0.184007 0.372071,0.256315 m 0.173632,-0.777214 c -0.04786,0.192239 0.105133,0.331995 0.157097,0.496094 m 0.677995,-0.644922 c -0.28271,0.05203 -0.181647,0.368968 -0.264584,0.537435 m 0.628386,-0.272851 c -0.158369,0.145487 -0.2579,0.347605 -0.380339,0.51263 m -1.339453,-0.735873 c 0.241692,0.350867 0.440936,0.727371 0.661458,1.091407 m -0.520898,0.115755 c -0.12251,-0.280055 -0.418817,-0.152812 -0.611849,-0.223242"/>
  <circle style="fill:HAIR_COLOR;fill-opacity:1;stroke:HAIR_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" cx="19.319094" cy="13.540565" r="0.97978514"/>
  <path style="fill:HAIR_COLOR;fill-opacity:1;stroke:HAIR_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" d="m 19.203766,12.039882 c -0.178986,0.162759 -0.08483,0.418474 -0.124024,0.611849"/>
</g>"#;

// ===========================================================================
// Shirts (4 hand-drawn)
// ===========================================================================

const SHIRT_CREW: &str = r#"<g id="shirt">
  <path style="fill:SHIRT_COLOR;fill-opacity:1;stroke:none;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 21.127463,26.624733 c -0.833097,0.696632 -2.774842,0.851607 -3.906739,-0.0036 -4.057841,0.336216 -4.366944,1.766888 -4.511869,4.921147 l 13.459644,0.02894 c -0.309965,-2.899166 -0.674136,-4.609096 -5.036902,-4.950602 z"/>
</g>"#;

const SHIRT_ROUND: &str = r#"<g id="shirt">
  <path style="fill:SHIRT_COLOR;fill-opacity:1;stroke:none;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 21.127463,26.624733 c -0.254321,2.416424 -3.403227,3.282466 -3.906739,-0.0036 -4.057841,0.336216 -4.366944,1.766888 -4.511869,4.921147 l 13.459644,0.02894 c -0.309965,-2.899166 -0.674136,-4.609096 -5.036902,-4.950602 z"/>
</g>"#;

const SHIRT_TURTLENECK: &str = r#"<g id="shirt">
  <path style="fill:SHIRT_COLOR;fill-opacity:1;stroke:none;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" d="m 17.260733,25.82302 c 0.01778,0.154745 -8.73e-4,0.454755 -0.03772,0.814421 -8.5e-4,-0.0054 -0.0012,-0.01108 -0.0021,-0.01654 -4.057841,0.336216 -4.366944,1.766888 -4.511869,4.921147 l 13.459644,0.02894 c -0.309856,-2.898142 -0.67385,-4.607734 -5.032251,-4.950086 0.0041,-0.238803 0.01895,-0.423915 0.04857,-0.642338 -1.097165,0.390776 -2.879558,0.505753 -3.924308,-0.155546 z"/>
</g>"#;

// Portrait 03 shirt — wrapped in translate(-28,0)
const SHIRT_VNECK: &str = r#"<g id="shirt" transform="translate(-28,0)">
  <path style="fill:SHIRT_COLOR;fill-opacity:1;stroke:none;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 45.220278,26.768621 c -4.057841,0.336216 -5.73503,1.743502 -5.879955,4.897761 l 15.669629,0.0056 c -0.309965,-2.899166 -2.512285,-4.865928 -5.901971,-4.98215 -0.375252,0.296254 -1.138617,2.431458 -1.875243,2.484003 -0.736626,0.05254 -1.446512,-1.977567 -2.01246,-2.405173 z"/>
</g>"#;

// ===========================================================================
// Accessories (4 hand-drawn)
// ===========================================================================

const ACC_NECKLACE: &str = r#"<g id="accessory">
  <circle style="fill:ACC_COLOR;fill-opacity:1;stroke:ACC_COLOR;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" cx="19.260839" cy="29.645733" r="0.5"/>
  <path style="fill:ACC_COLOR;stroke:#ffffff;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" d="m 19.028375,29.413199 0.464928,0.464934"/>
  <path style="fill:ACC_COLOR;stroke:#ffffff;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" d="m 19.485328,29.421303 -0.448865,0.44886"/>
  <path style="fill:none;stroke:ACC_COLOR;stroke-width:0.25;stroke-linecap:round;stroke-linejoin:round" d="m 17.024284,26.698112 c 0.590675,0.982232 1.416073,1.585167 2.207617,2.447396 0.04775,-0.177054 1.056187,-0.923888 2.116666,-2.463933"/>
</g>"#;

const ACC_SPIKED_COLLAR: &str = r#"<g id="accessory" transform="translate(-27.867259,-0.04827012)">
  <ellipse style="fill:none;stroke:ACC_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" cx="40.117485" cy="33.74147" rx="0.2" ry="0.40000001" transform="rotate(-8.0500371)"/>
  <path style="fill:none;stroke:ACC_COLOR;stroke-width:0.191516;stroke-linecap:round;stroke-linejoin:round" d="m 44.481226,28.031201 c 0.207384,0.741874 0.204398,0.732398 0.204398,0.732398"/>
  <ellipse style="fill:none;stroke:ACC_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" cx="28.159096" cy="45.329445" rx="0.2" ry="0.40000001" transform="rotate(-25.205399)"/>
  <path style="fill:none;stroke:ACC_COLOR;stroke-width:0.191516;stroke-linecap:round;stroke-linejoin:round" d="m 44.879304,29.247832 c 0.409267,0.652599 0.403717,0.644358 0.403717,0.644358"/>
  <ellipse style="fill:none;stroke:ACC_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" cx="9.9279919" cy="53.539112" rx="0.2" ry="0.40000001" transform="rotate(-46.009544)"/>
  <path style="fill:none;stroke:ACC_COLOR;stroke-width:0.191516;stroke-linecap:round;stroke-linejoin:round" d="m 45.576536,30.215061 c 0.663544,0.391273 0.654825,0.386508 0.654825,0.386508"/>
  <ellipse style="fill:none;stroke:ACC_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" cx="-16.66218" cy="53.177891" rx="0.2" ry="0.40000001" transform="rotate(-73.956899)"/>
  <path style="fill:none;stroke:ACC_COLOR;stroke-width:0.191516;stroke-linecap:round;stroke-linejoin:round" d="m 46.688952,30.763781 c 0.770182,0.01439 0.760247,0.01453 0.760247,0.01453"/>
  <path style="fill:none;stroke:ACC_COLOR;stroke-width:0.191516;stroke-linecap:round;stroke-linejoin:round" d="m 44.438082,26.762561 c -0.02525,0.769901 -0.02525,0.759965 -0.02525,0.759965"/>
  <ellipse style="fill:none;stroke:ACC_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" cx="-53.060871" cy="20.566416" rx="0.2" ry="0.40000001" transform="matrix(-0.99014615,-0.14003786,-0.14003786,0.99014615,0,0)"/>
  <path style="fill:none;stroke:ACC_COLOR;stroke-width:0.191516;stroke-linecap:round;stroke-linejoin:round" d="m 49.62397,28.03447 c -0.207384,0.741874 -0.204398,0.732398 -0.204398,0.732398"/>
  <ellipse style="fill:none;stroke:ACC_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" cx="-56.98745" cy="5.2563357" rx="0.2" ry="0.40000001" transform="matrix(-0.90478693,-0.42586455,-0.42586455,0.90478693,0,0)"/>
  <path style="fill:none;stroke:ACC_COLOR;stroke-width:0.191516;stroke-linecap:round;stroke-linejoin:round" d="m 49.225892,29.251101 c -0.409267,0.652599 -0.403717,0.644358 -0.403717,0.644358"/>
  <ellipse style="fill:none;stroke:ACC_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" cx="-55.434044" cy="-14.163116" rx="0.2" ry="0.40000001" transform="matrix(-0.69453854,-0.7194555,-0.7194555,0.69453854,0,0)"/>
  <path style="fill:none;stroke:ACC_COLOR;stroke-width:0.191516;stroke-linecap:round;stroke-linejoin:round" d="m 48.52866,30.21833 c -0.663544,0.391273 -0.654825,0.386508 -0.654825,0.386508"/>
  <ellipse style="fill:none;stroke:ACC_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" cx="-42.672272" cy="-37.261383" rx="0.2" ry="0.40000001" transform="matrix(-0.27636039,-0.96105407,-0.96105407,0.27636039,0,0)"/>
  <path style="fill:none;stroke:ACC_COLOR;stroke-width:0.191516;stroke-linecap:round;stroke-linejoin:round" d="m 49.667114,26.76583 c 0.02525,0.769901 0.02525,0.759965 0.02525,0.759965"/>
</g>"#;

// Portrait 03 piercings — wrapped in translate(-28,0)
const ACC_PIERCINGS: &str = r#"<g id="accessory" transform="translate(-28,0)">
  <circle style="fill:EYE_COLOR;fill-opacity:1;stroke:none;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" cx="52.362694" cy="21.468456" r="0.30000001"/>
  <circle style="fill:EYE_COLOR;fill-opacity:1;stroke:none;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" cx="42.550648" cy="21.235157" r="0.30000001"/>
</g>"#;

const ACC_EARRING: &str = r#"<g id="accessory">
  <path style="fill:none;stroke:ACC_COLOR;stroke-width:0.2;stroke-linecap:round;stroke-linejoin:round" d="m 24.058751,20.885428 c -0.05722,0.205569 -0.142447,0.600772 -0.01754,0.684043 0.191672,0.127781 0.263386,-0.234767 0.263386,-0.345949"/>
</g>"#;

const ACC_NONE: &str = r#"<g id="accessory"/>"#;

// ===========================================================================
// Color helpers
// ===========================================================================

/// Darken a color to ~70% lightness for shadow tones.
pub fn compute_shadow(base: [f32; 3]) -> [f32; 3] {
    [base[0] * 0.7, base[1] * 0.7, base[2] * 0.7]
}

/// Brighten a color to ~130% lightness for highlight tones, clamped to 1.0.
pub fn compute_highlight(base: [f32; 3]) -> [f32; 3] {
    [
        (base[0] * 1.3).min(1.0),
        (base[1] * 1.3).min(1.0),
        (base[2] * 1.3).min(1.0),
    ]
}

fn color_to_hex(rgb: [f32; 3]) -> String {
    let r = (rgb[0].clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (rgb[1].clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (rgb[2].clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}

// ===========================================================================
// Fragment selection helpers
// ===========================================================================

fn face_fragment(shape: &FaceShape) -> &'static str {
    match shape {
        FaceShape::Oval => FACE_OVAL,
        FaceShape::Round => FACE_ROUND,
        FaceShape::Square => FACE_SQUARE,
        FaceShape::Angular => FACE_ANGULAR,
        FaceShape::Long => FACE_OVAL,       // fallback
        FaceShape::Diamond => FACE_ANGULAR,  // fallback
    }
}

fn eyes_fragment(style: &EyeStyle) -> &'static str {
    match style {
        EyeStyle::Normal => EYES_NORMAL,
        EyeStyle::Narrow => EYES_NARROW,
        EyeStyle::Wide => EYES_WIDE,
        EyeStyle::Visor => EYES_VISOR,
        EyeStyle::Goggles => EYES_WIDE,     // fallback
        EyeStyle::Winking => EYES_NORMAL,   // fallback
    }
}

fn mouth_fragment(style: &MouthStyle) -> &'static str {
    match style {
        MouthStyle::Neutral => MOUTH_NEUTRAL,
        MouthStyle::Smile => MOUTH_SMILE,
        MouthStyle::Smirk => MOUTH_SMIRK,
        MouthStyle::Gritted => MOUTH_FROWN,  // fallback
        MouthStyle::Frown => MOUTH_FROWN,
    }
}

fn hair_back_fragment(style: &HairStyle) -> &'static str {
    match style {
        HairStyle::ShortCrop => HAIR_SHORT_CROP_BACK,
        HairStyle::Mohawk => "",
        HairStyle::LongSwept => HAIR_LONG_SWEPT_BACK,
        HairStyle::Helmet => HAIR_BEANIE_BACK,       // fallback
        HairStyle::Beanie => HAIR_BEANIE_BACK,
        HairStyle::Bald => HAIR_SHORT_CROP_BACK,     // fallback
        HairStyle::Ponytail => HAIR_LONG_SWEPT_BACK,  // fallback
    }
}

fn hair_front_fragment(style: &HairStyle) -> &'static str {
    match style {
        HairStyle::ShortCrop => HAIR_SHORT_CROP_FRONT,
        HairStyle::Mohawk => HAIR_MOHAWK_FRONT,
        HairStyle::LongSwept => HAIR_LONG_SWEPT_FRONT,
        HairStyle::Helmet => HAIR_BEANIE_FRONT,        // fallback
        HairStyle::Beanie => HAIR_BEANIE_FRONT,
        HairStyle::Bald => HAIR_SHORT_CROP_FRONT,      // fallback
        HairStyle::Ponytail => HAIR_LONG_SWEPT_FRONT,   // fallback
    }
}

fn shirt_fragment(style: &ShirtStyle) -> &'static str {
    match style {
        ShirtStyle::Crew => SHIRT_CREW,
        ShirtStyle::Round => SHIRT_ROUND,
        ShirtStyle::Turtleneck => SHIRT_TURTLENECK,
        ShirtStyle::Vneck => SHIRT_VNECK,
    }
}

fn accessory_fragment(acc: Option<&Accessory>) -> &'static str {
    match acc {
        None => ACC_NONE,
        Some(Accessory::Necklace) => ACC_NECKLACE,
        Some(Accessory::SpikedCollar) => ACC_SPIKED_COLLAR,
        Some(Accessory::Piercings) => ACC_PIERCINGS,
        Some(Accessory::Earring) => ACC_EARRING,
    }
}

// ===========================================================================
// SVG assembly
// ===========================================================================

/// Assemble a complete portrait SVG from a descriptor and background color.
///
/// Layer order: background rect -> hair back -> face -> shirt -> eyes ->
///              mouth -> hair front -> accessory.
pub fn assemble_svg(descriptor: &PortraitDescriptor, bg_color: [f32; 3]) -> String {
    let skin_hex = color_to_hex(descriptor.skin_tone);
    let skin_shadow_hex = color_to_hex(compute_shadow(descriptor.skin_tone));
    let skin_highlight_hex = color_to_hex(compute_highlight(descriptor.skin_tone));
    let hair_hex = color_to_hex(descriptor.hair_color);
    let hair_shadow_hex = color_to_hex(compute_shadow(descriptor.hair_color));
    let eye_hex = color_to_hex(descriptor.eye_color);
    let bg_hex = color_to_hex(bg_color);
    let acc_hex = color_to_hex(descriptor.accessory_color);
    let acc_shadow_hex = color_to_hex(compute_shadow(descriptor.accessory_color));
    let shirt_hex = color_to_hex(descriptor.shirt_color);

    let hair_back = hair_back_fragment(&descriptor.hair);
    let face = face_fragment(&descriptor.face_shape);
    let shirt = shirt_fragment(&descriptor.shirt);
    let eyes = eyes_fragment(&descriptor.eyes);
    let mouth = mouth_fragment(&descriptor.mouth);
    let hair_front = hair_front_fragment(&descriptor.hair);
    let accessory = accessory_fragment(descriptor.accessory.as_ref());

    let mut svg = String::with_capacity(8192);
    svg.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="9.5 11.5 20.1 20.1">"#);

    svg.push_str(&format!(
        r#"<rect x="9.5" y="11.5" width="20.1" height="20.1" rx="1.5" ry="1.5" fill="{bg_hex}"/>"#
    ));

    if !hair_back.is_empty() {
        svg.push_str(hair_back);
    }
    svg.push_str(face);
    svg.push_str(shirt);
    svg.push_str(eyes);
    svg.push_str(mouth);
    svg.push_str(hair_front);
    svg.push_str(accessory);

    svg.push_str("</svg>");

    let svg = svg.replace(SKIN_HIGHLIGHT, &skin_highlight_hex);
    let svg = svg.replace(SKIN_SHADOW, &skin_shadow_hex);
    let svg = svg.replace(SKIN_TONE, &skin_hex);
    let svg = svg.replace(HAIR_SHADOW, &hair_shadow_hex);
    let svg = svg.replace(HAIR_COLOR, &hair_hex);
    let svg = svg.replace(EYE_COLOR, &eye_hex);
    let svg = svg.replace(BG_COLOR, &bg_hex);
    let svg = svg.replace(ACC_SHADOW, &acc_shadow_hex);
    let svg = svg.replace(ACC_COLOR, &acc_hex);
    svg.replace(SHIRT_COLOR, &shirt_hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_to_hex_black() {
        assert_eq!(color_to_hex([0.0, 0.0, 0.0]), "#000000");
    }

    #[test]
    fn color_to_hex_white() {
        assert_eq!(color_to_hex([1.0, 1.0, 1.0]), "#ffffff");
    }

    #[test]
    fn color_to_hex_red() {
        assert_eq!(color_to_hex([1.0, 0.0, 0.0]), "#ff0000");
    }

    #[test]
    fn color_to_hex_clamped() {
        assert_eq!(color_to_hex([1.5, -0.5, 0.5]), "#ff0080");
    }

    #[test]
    fn compute_shadow_darkens() {
        let base = [1.0, 0.8, 0.6];
        let shadow = compute_shadow(base);
        assert!((shadow[0] - 0.7).abs() < 1e-6);
        assert!((shadow[1] - 0.56).abs() < 1e-6);
        assert!((shadow[2] - 0.42).abs() < 1e-6);
    }

    #[test]
    fn compute_highlight_brightens_and_clamps() {
        let base = [0.5, 0.9, 1.0];
        let highlight = compute_highlight(base);
        assert!((highlight[0] - 0.65).abs() < 1e-6);
        assert!((highlight[1] - 1.0).abs() < 1e-6);
        assert!((highlight[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn assemble_svg_contains_svg_wrapper() {
        let desc = PortraitDescriptor {
            face_shape: FaceShape::Oval,
            eyes: EyeStyle::Normal,
            mouth: MouthStyle::Neutral,
            hair: HairStyle::ShortCrop,
            shirt: ShirtStyle::Crew,
            accessory: None,
            skin_tone: [0.9, 0.7, 0.55],
            hair_color: [0.2, 0.15, 0.1],
            eye_color: [0.2, 0.5, 0.8],
            accessory_color: [0.5, 0.5, 0.5],
            shirt_color: [0.8, 0.8, 0.85],
            generated: true,
        };
        let svg = assemble_svg(&desc, [0.1, 0.1, 0.2]);
        assert!(svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="9.5 11.5 20.1 20.1">"#));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn assemble_svg_no_placeholders_remain() {
        let desc = PortraitDescriptor {
            face_shape: FaceShape::Angular,
            eyes: EyeStyle::Visor,
            mouth: MouthStyle::Frown,
            hair: HairStyle::Beanie,
            shirt: ShirtStyle::Turtleneck,
            accessory: Some(Accessory::SpikedCollar),
            skin_tone: [0.85, 0.65, 0.5],
            hair_color: [0.3, 0.3, 0.3],
            eye_color: [0.1, 0.8, 0.1],
            accessory_color: [0.9, 0.1, 0.1],
            shirt_color: [0.8, 0.75, 0.9],
            generated: true,
        };
        let svg = assemble_svg(&desc, [0.05, 0.05, 0.15]);

        assert!(!svg.contains(SKIN_TONE), "SKIN_TONE placeholder remains");
        assert!(!svg.contains(SKIN_SHADOW), "SKIN_SHADOW placeholder remains");
        assert!(!svg.contains(SKIN_HIGHLIGHT), "SKIN_HIGHLIGHT placeholder remains");
        assert!(!svg.contains(HAIR_COLOR), "HAIR_COLOR placeholder remains");
        assert!(!svg.contains(HAIR_SHADOW), "HAIR_SHADOW placeholder remains");
        assert!(!svg.contains(EYE_COLOR), "EYE_COLOR placeholder remains");
        assert!(!svg.contains(BG_COLOR), "BG_COLOR placeholder remains");
        assert!(!svg.contains(ACC_COLOR), "ACC_COLOR placeholder remains");
        assert!(!svg.contains(ACC_SHADOW), "ACC_SHADOW placeholder remains");
        assert!(!svg.contains(SHIRT_COLOR), "SHIRT_COLOR placeholder remains");
    }

    #[test]
    fn assemble_svg_contains_all_layer_groups() {
        let desc = PortraitDescriptor {
            face_shape: FaceShape::Angular,
            eyes: EyeStyle::Wide,
            mouth: MouthStyle::Smile,
            hair: HairStyle::ShortCrop,
            shirt: ShirtStyle::Round,
            accessory: Some(Accessory::Necklace),
            skin_tone: [0.8, 0.6, 0.4],
            hair_color: [0.1, 0.1, 0.1],
            eye_color: [0.3, 0.3, 0.7],
            accessory_color: [0.7, 0.2, 0.2],
            shirt_color: [0.8, 0.8, 0.8],
            generated: true,
        };
        let svg = assemble_svg(&desc, [0.0, 0.0, 0.0]);
        assert!(svg.contains(r#"id="face""#));
        assert!(svg.contains(r#"id="eyes""#));
        assert!(svg.contains(r#"id="mouth""#));
        assert!(svg.contains(r#"id="shirt""#));
        assert!(svg.contains(r#"id="accessory""#));
    }

    #[test]
    fn assemble_svg_hair_back_layer_present_for_long_swept() {
        let desc = PortraitDescriptor {
            face_shape: FaceShape::Oval,
            eyes: EyeStyle::Normal,
            mouth: MouthStyle::Neutral,
            hair: HairStyle::LongSwept,
            shirt: ShirtStyle::Crew,
            accessory: None,
            skin_tone: [0.9, 0.7, 0.55],
            hair_color: [0.4, 0.2, 0.1],
            eye_color: [0.2, 0.5, 0.8],
            accessory_color: [0.5, 0.5, 0.5],
            shirt_color: [0.8, 0.8, 0.85],
            generated: true,
        };
        let svg = assemble_svg(&desc, [0.1, 0.1, 0.2]);
        assert!(svg.contains(r#"id="hair-back""#));
    }

    #[test]
    fn assemble_svg_background_uses_bg_color() {
        let desc = PortraitDescriptor {
            face_shape: FaceShape::Square,
            eyes: EyeStyle::Narrow,
            mouth: MouthStyle::Frown,
            hair: HairStyle::Mohawk,
            shirt: ShirtStyle::Vneck,
            accessory: Some(Accessory::Earring),
            skin_tone: [0.8, 0.6, 0.5],
            hair_color: [0.9, 0.1, 0.5],
            eye_color: [0.1, 0.1, 0.9],
            accessory_color: [0.8, 0.8, 0.0],
            shirt_color: [0.8, 0.8, 0.85],
            generated: true,
        };
        let bg = [0.0, 1.0, 0.0];
        let svg = assemble_svg(&desc, bg);
        let bg_hex = color_to_hex(bg);
        assert!(svg.contains(&format!(r#"fill="{bg_hex}""#)));
    }

    #[test]
    fn all_face_fragments_valid() {
        for shape in &[
            FaceShape::Oval, FaceShape::Round, FaceShape::Square,
            FaceShape::Angular, FaceShape::Long, FaceShape::Diamond,
        ] {
            let frag = face_fragment(shape);
            assert!(frag.contains("<g"), "{shape:?} missing <g");
            assert!(frag.contains("</g>"), "{shape:?} missing </g>");
            assert!(frag.contains(SKIN_TONE), "{shape:?} missing SKIN_TONE");
        }
    }

    #[test]
    fn all_eye_fragments_valid() {
        for style in &[
            EyeStyle::Normal, EyeStyle::Narrow, EyeStyle::Wide,
            EyeStyle::Visor, EyeStyle::Goggles, EyeStyle::Winking,
        ] {
            let frag = eyes_fragment(style);
            assert!(frag.contains("<g"), "{style:?} missing <g");
            assert!(frag.contains("</g>"), "{style:?} missing </g>");
            assert!(
                frag.contains(EYE_COLOR) || frag.contains(SKIN_HIGHLIGHT),
                "{style:?} missing eye color token"
            );
        }
    }

    #[test]
    fn all_mouth_fragments_valid() {
        for style in &[
            MouthStyle::Neutral, MouthStyle::Smile, MouthStyle::Smirk,
            MouthStyle::Gritted, MouthStyle::Frown,
        ] {
            let frag = mouth_fragment(style);
            assert!(frag.contains("<g"), "{style:?} missing <g");
            assert!(frag.contains("</g>"), "{style:?} missing </g>");
            assert!(
                frag.contains(SKIN_SHADOW) || frag.contains(SKIN_HIGHLIGHT),
                "{style:?} missing skin color token"
            );
        }
    }

    #[test]
    fn all_hair_front_fragments_nonempty() {
        for style in &[
            HairStyle::ShortCrop, HairStyle::Mohawk, HairStyle::LongSwept,
            HairStyle::Helmet, HairStyle::Beanie, HairStyle::Bald, HairStyle::Ponytail,
        ] {
            let frag = hair_front_fragment(style);
            assert!(!frag.is_empty(), "{style:?} front fragment is empty");
        }
    }

    #[test]
    fn hair_back_layers_for_expected_styles() {
        let back_styles = [
            HairStyle::ShortCrop, HairStyle::LongSwept, HairStyle::Beanie,
            HairStyle::Helmet, HairStyle::Bald, HairStyle::Ponytail,
        ];
        for style in &back_styles {
            assert!(
                !hair_back_fragment(style).is_empty(),
                "{style:?} should have a back layer"
            );
        }
        assert!(
            hair_back_fragment(&HairStyle::Mohawk).is_empty(),
            "Mohawk should NOT have a back layer"
        );
    }

    #[test]
    fn all_shirt_fragments_valid() {
        for style in &[
            ShirtStyle::Crew, ShirtStyle::Round,
            ShirtStyle::Turtleneck, ShirtStyle::Vneck,
        ] {
            let frag = shirt_fragment(style);
            assert!(frag.contains("<g"), "{style:?} missing <g");
            assert!(frag.contains("</g>"), "{style:?} missing </g>");
            assert!(frag.contains(SHIRT_COLOR), "{style:?} missing SHIRT_COLOR");
        }
    }

    #[test]
    fn all_accessory_fragments_valid() {
        let frag = accessory_fragment(None);
        assert!(frag.contains("<g"), "None missing <g");

        for acc in &[
            Accessory::Necklace, Accessory::SpikedCollar,
            Accessory::Piercings, Accessory::Earring,
        ] {
            let frag = accessory_fragment(Some(acc));
            assert!(frag.contains("<g"), "{acc:?} missing <g");
        }
    }

    #[test]
    fn layer_order_correct() {
        let desc = PortraitDescriptor {
            face_shape: FaceShape::Oval,
            eyes: EyeStyle::Normal,
            mouth: MouthStyle::Smile,
            hair: HairStyle::LongSwept,
            shirt: ShirtStyle::Crew,
            accessory: Some(Accessory::Necklace),
            skin_tone: [0.8, 0.6, 0.45],
            hair_color: [0.3, 0.15, 0.05],
            eye_color: [0.2, 0.6, 0.3],
            accessory_color: [0.7, 0.2, 0.2],
            shirt_color: [0.8, 0.8, 0.85],
            generated: true,
        };
        let svg = assemble_svg(&desc, [0.1, 0.1, 0.15]);

        let bg_pos = svg.find("rx=\"1.5\"").unwrap();
        let hair_back_pos = svg.find(r#"id="hair-back""#).unwrap();
        let face_pos = svg.find(r#"id="face""#).unwrap();
        let shirt_pos = svg.find(r#"id="shirt""#).unwrap();
        let eyes_pos = svg.find(r#"id="eyes""#).unwrap();
        let mouth_pos = svg.find(r#"id="mouth""#).unwrap();
        let hair_front_pos = svg.find(r#"id="hair""#).unwrap();
        let acc_pos = svg.find(r#"id="accessory""#).unwrap();

        assert!(bg_pos < hair_back_pos, "bg before hair-back");
        assert!(hair_back_pos < face_pos, "hair-back before face");
        assert!(face_pos < shirt_pos, "face before shirt");
        assert!(shirt_pos < eyes_pos, "shirt before eyes");
        assert!(eyes_pos < mouth_pos, "eyes before mouth");
        assert!(mouth_pos < hair_front_pos, "mouth before hair-front");
        assert!(hair_front_pos < acc_pos, "hair-front before accessory");
    }
}
