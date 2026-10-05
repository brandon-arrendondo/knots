-- probe: if / elsif / else
-- source: McCabe 1976 (if and elsif are decisions); Cognitive 1.7 B1 (if, else if, else each +1)
-- expect F mccabe=3 cognitive=3
function F (X : Integer) return Integer is
begin
   if X > 0 then
      return 1;
   elsif X < 0 then
      return -1;
   else
      return 0;
   end if;
end F;
