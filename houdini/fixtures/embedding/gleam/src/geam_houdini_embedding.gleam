import houdini

pub fn verify() -> Bool {
  assert houdini.escape("<") == "&lt;"
  assert houdini.escape(">") == "&gt;"
  assert houdini.escape("&") == "&amp;"
  assert houdini.escape("\"") == "&quot;"
  assert houdini.escape("'") == "&#39;"
  assert houdini.escape("<>&\"'") == "&lt;&gt;&amp;&quot;&#39;"
  assert houdini.escape("") == ""
  assert houdini.escape("&lt;") == "&amp;lt;"
  assert houdini.escape("plain 한글 🪄") == "plain 한글 🪄"
  assert houdini.escape("A&한글<B") == "A&amp;한글&lt;B"
  assert houdini.escape("abcdefgh<ijklmnop") == "abcdefgh&lt;ijklmnop"
  assert houdini.escape("1234567<") == "1234567&lt;"
  assert houdini.escape("12345678<") == "12345678&lt;"
  assert houdini.escape("<a x=\"é & '한글'\">")
    == "&lt;a x=&quot;é &amp; &#39;한글&#39;&quot;&gt;"
  True
}

pub fn escape(value: String) -> String {
  houdini.escape(value)
}
