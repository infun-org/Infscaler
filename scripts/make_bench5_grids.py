#!/usr/bin/env python3
"""Three-column original-lanczos / enhance / face pipeline grids."""
from pathlib import Path
import sys
from PIL import Image, ImageDraw, ImageFont

root=Path(__file__).resolve().parents[1]
out=Path(sys.argv[1]) if len(sys.argv)>1 else root/'samples/out/bench5'
names=('portrait','portrait-blur','person','child','cat','dog','horse')
labels=('original (Lanczos3 to output size)','enhance only','enhance + face restore')
# Face centers in source-image pixels make restoration differences visible.
centers={'portrait':(225,42),'portrait-blur':(225,42),'person':(132,92),
         'child':(120,92),'cat':(220,68),'dog':(200,125),'horse':(280,130)}
font=ImageFont.load_default(); gutter=8; label_h=24
for name in names:
    src=Image.open(root/'samples/real'/f'{name}.jpg').convert('RGB')
    enhanced=Image.open(out/f'{name}__enhance.png').convert('RGB')
    restored=Image.open(out/f'{name}__face.png').convert('RGB')
    original=src.resize(enhanced.size,Image.Resampling.LANCZOS)
    panels=[]
    for im,label in zip((original,enhanced,restored),labels):
        w,h=im.size; side=min(400,w,h)
        sx=w/src.width; sy=h/src.height
        cx,cy=centers[name]
        x=max(0,min(w-side,int((cx-50)*sx))); y=max(0,min(h-side,int((cy-50)*sy)))
        crop=im.crop((x,y,x+side,y+side)); tile=Image.new('RGB',(side,label_h+side),'white'); tile.paste(crop,(0,label_h))
        ImageDraw.Draw(tile).text((4,5),label,fill=(20,20,20),font=font);panels.append(tile)
    grid=Image.new('RGB',(sum(p.width for p in panels)+gutter*2,panels[0].height),'white')
    x=0
    for p in panels:grid.paste(p,(x,0));x+=p.width+gutter
    grid.save(out/f'compare_{name}.jpg',quality=90)
    print(out/f'compare_{name}.jpg')
