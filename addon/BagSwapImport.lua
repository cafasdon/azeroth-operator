local _, AO = ...
local function number(v,lo,hi)
    assert(v:match('^%d+$') and #v<=10 and (v=='0' or v:sub(1,1)~='0'),'Invalid swap number')
    local n=tonumber(v);assert(n>=lo and n<=hi,'Swap number out of range');return n
end
local function text(v,max)
    assert(#v>0 and #v%2==0 and #v<=max*2 and not v:find('[^0-9a-f]'),'Invalid swap text')
    local decoded=v:gsub('..',function(pair) return string.char(tonumber(pair,16)) end)
    assert(not decoded:find('%c'),'Control character in swap text');return decoded
end
function AO.DecodeBagSwap(code)
    assert(type(code)=='string' and #code<=8192,'Invalid swap size')
    local f={};for value in (code..':'):gmatch('([^:]*):') do f[#f+1]=value end
    assert(#f==21 and f[1]=='AO-BAGSWAP-1' and f[4]=='retail','Invalid swap format')
    for _,i in ipairs({2,3}) do assert(#f[i]==64 and not f[i]:find('[^0-9a-f]'),'Invalid swap identity') end
    local function entry(i)
        return {bag=number(f[i],0,64),slot=number(f[i+1],1,1024),item_id=number(f[i+2],1,4294967295),
            count=number(f[i+3],1,2147483647),guid=text(f[i+4],128),link=text(f[i+5],4096)}
    end
    local op={id=f[2],snapshot_id=f[3],family=f[4],variant=text(f[5],128),build=text(f[6],128),
        interface=number(f[7],1,4294967295),character=text(f[8],128),realm=text(f[9],128),
        kind='swap_bag_items',source=entry(10),target=entry(16)}
    assert(op.source.bag~=op.target.bag or op.source.slot~=op.target.slot,'Swap locations equal')
    assert(op.source.guid~=op.target.guid and op.source.item_id~=op.target.item_id,'Swap identities must differ')
    return op
end
